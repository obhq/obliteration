use self::hv::{Cpu, CpuExit, CpuStates, Hypervisor};
use self::ram::Ram;
use self::screen::Screen;
use crate::error::RustError;
use obconf::{BootEnv, Vm};
use obvirt::console::MsgType;
use std::collections::VecDeque;
use std::error::Error;
use std::ffi::{c_char, c_void, CStr};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use thiserror::Error;

mod hv;
mod ram;
mod screen;

#[cfg(target_arch = "x86_64")]
const ELF_MACHINE: u16 = 62;
#[cfg(target_arch = "aarch64")]
const ELF_MACHINE: u16 = 183;

#[no_mangle]
pub unsafe extern "C" fn vmm_new(screen: usize, err: *mut *mut RustError) -> *mut Vmm {
    // Setup RAM.
    let ram = match Ram::new() {
        Ok(v) => Arc::new(v),
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Setup hypervisor.
    let hv = match self::hv::Default::new(8, ram.clone()) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Setup screen.
    let screen = match self::screen::Default::new(screen) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Create VMM.
    let vmm = Vmm {
        hv: Arc::new(hv),
        ram,
        cpus: Vec::new(),
        screen,
        logs: Arc::default(),
        shutdown: Arc::new(AtomicBool::new(false)),
    };

    Box::into_raw(vmm.into())
}

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_run(vmm: *mut Vmm, kernel: *const c_char) -> *mut RustError {
    if !(*vmm).cpus.is_empty() {
        return RustError::new("the kernel is already running");
    }

    // Check if path UTF-8.
    let path = match CStr::from_ptr(kernel).to_str() {
        Ok(v) => v,
        Err(_) => return RustError::new("path of the kernel is not UTF-8"),
    };

    // Open kernel image.
    let mut file = match File::open(path) {
        Ok(v) => v,
        Err(e) => return RustError::with_source("couldn't open the kernel", e),
    };

    // Read file header.
    let mut hdr = [0; 64];

    if let Err(e) = file.read_exact(&mut hdr) {
        return RustError::with_source("couldn't read kernel header", e);
    }

    // Check if ELF.
    if &hdr[..4] != b"\x7fELF" {
        return RustError::new("the kernel is not an ELF file");
    }

    // Check ELF type.
    if hdr[4] != 2 {
        return RustError::new("the kernel is not 64-bit kernel");
    }

    if hdr[6] != 1 {
        return RustError::new("the kernel has unknown ELF version");
    }

    if u16::from_ne_bytes(hdr[18..20].try_into().unwrap()) != ELF_MACHINE {
        return RustError::new("the kernel is for a different CPU architecture");
    }

    // Load ELF header.
    let e_entry = usize::from_ne_bytes(hdr[24..32].try_into().unwrap());
    let e_phoff = u64::from_ne_bytes(hdr[32..40].try_into().unwrap());
    let e_phentsize: usize = u16::from_ne_bytes(hdr[54..56].try_into().unwrap()).into();
    let e_phnum: usize = u16::from_ne_bytes(hdr[56..58].try_into().unwrap()).into();

    if e_phentsize != 56 {
        return RustError::new("the kernel has unsupported e_phentsize");
    }

    if e_phnum > 16 {
        return RustError::new("too many program headers");
    }

    // Seek to program headers.
    match file.seek(SeekFrom::Start(e_phoff)) {
        Ok(v) => {
            if v != e_phoff {
                return RustError::new("the kernel is incomplete");
            }
        }
        Err(e) => return RustError::with_source("couldn't seek to program headers", e),
    }

    // Read program headers.
    let mut data = vec![0; e_phnum * e_phentsize];

    if let Err(e) = file.read_exact(&mut data) {
        return RustError::with_source("couldn't read program headers", e);
    }

    // Parse program headers.
    let mut loads = Vec::with_capacity(e_phnum);
    let mut dynamic = None;

    for (index, data) in data.chunks_exact(e_phentsize).enumerate() {
        let p_type = u32::from_ne_bytes(data[..4].try_into().unwrap());
        let p_offset = u64::from_ne_bytes(data[8..16].try_into().unwrap());
        let p_vaddr = usize::from_ne_bytes(data[16..24].try_into().unwrap());
        let p_filesz = usize::from_ne_bytes(data[32..40].try_into().unwrap());
        let p_memsz = usize::from_ne_bytes(data[40..48].try_into().unwrap());
        let p_align = usize::from_ne_bytes(data[48..56].try_into().unwrap());

        match p_type {
            1 => {
                if p_filesz > p_memsz {
                    return RustError::new(format!("invalid p_filesz on on PT_LOAD {index}"));
                }

                if p_align != Ram::VM_PAGE_SIZE {
                    return RustError::new(format!("unsupported p_align on PT_LOAD {index}"));
                }

                loads.push((p_offset, p_filesz, p_vaddr, p_memsz));
            }
            2 => {
                if dynamic.is_some() {
                    return RustError::new("multiple PT_DYNAMIC is not supported");
                }

                dynamic = Some((p_vaddr, p_memsz));
            }
            6 | 1685382481 | 1685382482 => {}
            v => return RustError::new(format!("unknown p_type {v} on program header {index}")),
        }
    }

    loads.sort_unstable_by_key(|i| i.2);

    // Make sure the first PT_LOAD includes the ELF header.
    match loads.first() {
        Some(&(p_offset, _, _, _)) => {
            if p_offset != 0 {
                return RustError::new("the first PT_LOAD does not includes ELF header");
            }
        }
        None => return RustError::new("no any PT_LOAD on the kernel"),
    }

    // Get size of memory for the kernel.
    let mut end = 0;

    for &(_, _, p_vaddr, p_memsz) in &loads {
        if p_vaddr < end {
            return RustError::new("some PT_LOAD has overlapped");
        }

        end = p_vaddr + p_memsz;
    }

    end = end.next_multiple_of(Ram::VM_PAGE_SIZE);

    // Allocate RAM.
    let soff = end; // TODO: Figure out how PS4 allocate the stack.
    let slen = 1024 * 1024 * 2; // TODO: Same here.
    let len = soff + slen;
    let mem = match (*vmm).ram.alloc(0, len) {
        Ok(v) => v,
        Err(e) => {
            return RustError::with_source(format!("couldn't allocate {len} bytes from RAM"), e);
        }
    };

    // Map the kernel.
    for &(p_offset, p_filesz, p_vaddr, _) in &loads {
        // Seek to program data.
        match file.seek(SeekFrom::Start(p_offset)) {
            Ok(v) => {
                if v != p_offset {
                    return RustError::new("the kernel is incomplete");
                }
            }
            Err(e) => {
                return RustError::with_source(format!("couldn't seek to offset {p_offset}"), e);
            }
        }

        // Read program data.
        let dst = std::slice::from_raw_parts_mut(mem.add(p_vaddr), p_filesz);

        if let Err(e) = file.read_exact(dst) {
            return RustError::with_source(format!("couldn't read kernet at offset {p_offset}"), e);
        }
    }

    // Setup arguments for main CPU.
    let args = CpuArgs {
        hv: (*vmm).hv.clone(),
        screen: (*vmm).screen.buffer().clone(),
        logs: (*vmm).logs.clone(),
        shutdown: (*vmm).shutdown.clone(),
    };

    // Spawn a thread to drive main CPU.
    let ram = (*vmm).ram.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let main = move || main_cpu(&args, ram, end, slen, e_entry, dynamic.as_ref(), tx);
    let main = match std::thread::Builder::new().spawn(main) {
        Ok(v) => v,
        Err(e) => return RustError::with_source("couldn't spawn main CPU", e),
    };

    // Wait for main CPU to enter event loop.
    let r = match rx.recv() {
        Ok(v) => v,
        Err(_) => {
            main.join().unwrap();
            return RustError::new("main CPU stopped unexpectedly");
        }
    };

    if let Err(e) = r {
        main.join().unwrap();
        return RustError::with_source("couldn't start main CPU", e);
    }

    // Push to CPU list.
    (*vmm).cpus.push(main);

    null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn vmm_draw(vmm: *mut Vmm) -> *mut RustError {
    match (*vmm).screen.update() {
        Ok(_) => null_mut(),
        Err(e) => RustError::wrap(e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_logs(
    vmm: *const Vmm,
    cx: *mut c_void,
    cb: unsafe extern "C" fn(u8, *const c_char, usize, *mut c_void),
) {
    let logs = (*vmm).logs.lock().unwrap();

    for (ty, msg) in logs.deref() {
        cb(*ty as u8, msg.as_ptr().cast(), msg.len(), cx);
    }
}

/// # Safety
/// - The kernel must be mapped at address 0.
/// - The stack must be immediate follow the kernel.
unsafe fn main_cpu(
    args: &CpuArgs,
    ram: Arc<Ram>,
    klen: usize,
    slen: usize,
    entry: usize,
    dynamic: Option<&(usize, usize)>,
    status: Sender<Result<(), MainCpuError>>,
) {
    // Create vCPU.
    let mut cpu = match args.hv.create_cpu(0) {
        Ok(v) => v,
        Err(e) => {
            status.send(Err(MainCpuError::CreateCpuFailed(e))).unwrap();
            return;
        }
    };

    if let Err(e) = setup_main_cpu(&mut cpu, &ram, klen, slen, entry, dynamic) {
        status.send(Err(e)).unwrap();
        return;
    }

    // Enter dispatch loop.
    status.send(Ok(())).unwrap();
    drop(status);

    run_cpu(cpu, args);
}

/// # Safety
/// - The kernel must be mapped at address 0.
/// - The stack must be immediate follow the kernel.
#[cfg(target_arch = "x86_64")]
unsafe fn setup_main_cpu(
    cpu: &mut impl Cpu,
    ram: &Ram,
    klen: usize,
    slen: usize,
    entry: usize,
    dynamic: Option<&(usize, usize)>,
) -> Result<(), MainCpuError> {
    // For x86-64 we require the kernel to be a Position-Independent Executable so we can map it at
    // the same address as the PS4 kernel.
    let dynamic = dynamic.ok_or(MainCpuError::NonPieKernel)?;

    // Get total size of kernel arguments.
    let len = size_of::<BootEnv>().next_multiple_of(Ram::VM_PAGE_SIZE);

    assert!(align_of::<BootEnv>() <= Ram::VM_PAGE_SIZE);

    // Setup kernel arguments.
    let mut alloc = klen + slen;
    let kend = alloc + len;
    let args = ram
        .alloc(alloc, len)
        .map_err(MainCpuError::AllocArgsFailed)?;

    alloc += len;

    std::ptr::write(args.cast(), BootEnv::Vm(Vm {}));

    // Allocate page-map level-4 table. We use 4K 4-Level Paging here. Not sure how the PS4 achieve
    // 16K page because x86-64 does not support it. Maybe it is a special request from Sony to AMD?
    //
    // See Page Translation and Protection section on AMD64 Architecture Programmer's Manual Volume
    // 2 for how paging work in long-mode.
    let len = (512usize * 8).next_multiple_of(Ram::VM_PAGE_SIZE);
    let pml4t: &mut [usize; 512] = match ram.alloc(alloc, len) {
        Ok(v) => &mut *v.cast(),
        Err(e) => return Err(MainCpuError::AllocPml4TableFailed(e)),
    };

    alloc += len;

    // Setup page tables to map virtual address 0xffffffff82200000 to the kernel.
    // TODO: Implement ASLR.
    let base = 0xffffffff82200000;
    let end = base + kend; // kend is also size of the kernel because the kernel mapped at addr 0.

    for addr in (base..end).step_by(4096) {
        // Get page-directory pointer table.
        let pml4o = (addr & 0xFF8000000000) >> 39;
        let pdpt = match pml4t[pml4o] {
            0 => {
                // Allocate page-directory pointer table.
                let len = (512usize * 8).next_multiple_of(Ram::VM_PAGE_SIZE);
                let pdpt = match ram.alloc(alloc, len) {
                    Ok(v) => std::slice::from_raw_parts_mut::<usize>(v.cast(), 512),
                    Err(e) => return Err(MainCpuError::AllocPdpTableFailed(e)),
                };

                // Set page-map level-4 entry.
                assert_eq!(alloc & 0x7FF0000000000000, 0);
                assert_eq!(alloc & 0xFFF, 0);

                pml4t[pml4o] = alloc;
                pml4t[pml4o] |= 0b01; // Present (P) Bit.
                pml4t[pml4o] |= 0b10; // Read/Write (R/W) Bit.

                alloc += len;

                pdpt
            }
            v => std::slice::from_raw_parts_mut::<usize>(
                ram.host_addr().add(v & 0xFFFFFFFFFF000).cast_mut().cast(),
                512,
            ),
        };

        // Get page-directory table.
        let pdpo = (addr & 0x7FC0000000) >> 30;
        let pdt = match pdpt[pdpo] {
            0 => {
                // Allocate page-directory table.
                let len = (512usize * 8).next_multiple_of(Ram::VM_PAGE_SIZE);
                let pdt = match ram.alloc(alloc, len) {
                    Ok(v) => std::slice::from_raw_parts_mut::<usize>(v.cast(), 512),
                    Err(e) => return Err(MainCpuError::AllocPdTableFailed(e)),
                };

                assert_eq!(alloc & 0x7FF0000000000000, 0);
                assert_eq!(alloc & 0xFFF, 0);

                pdpt[pdpo] = alloc;
                pdpt[pdpo] |= 0b01; // Present (P) Bit.
                pdpt[pdpo] |= 0b10; // Read/Write (R/W) Bit.

                alloc += len;

                pdt
            }
            v => std::slice::from_raw_parts_mut::<usize>(
                ram.host_addr().add(v & 0xFFFFFFFFFF000).cast_mut().cast(),
                512,
            ),
        };

        // Get page table.
        let pdo = (addr & 0x3FE00000) >> 21;
        let pt = match pdt[pdo] {
            0 => {
                // Allocate page table.
                let len = (512usize * 8).next_multiple_of(Ram::VM_PAGE_SIZE);
                let pt = match ram.alloc(alloc, len) {
                    Ok(v) => std::slice::from_raw_parts_mut::<usize>(v.cast(), 512),
                    Err(e) => return Err(MainCpuError::AllocPageTableFailed(e)),
                };

                assert_eq!(alloc & 0x7FF0000000000000, 0);
                assert_eq!(alloc & 0xFFF, 0);

                pdt[pdo] = alloc;
                pdt[pdo] |= 0b01; // Present (P) Bit.
                pdt[pdo] |= 0b10; // Read/Write (R/W) Bit.

                alloc += len;

                pt
            }
            v => std::slice::from_raw_parts_mut::<usize>(
                ram.host_addr().add(v & 0xFFFFFFFFFF000).cast_mut().cast(),
                512,
            ),
        };

        // Set page table entry.
        let pto = (addr & 0x1FF000) >> 12;
        let addr = addr - base;

        assert_eq!(pt[pto], 0);
        assert_eq!(addr & 0x7FF0000000000000, 0);
        assert_eq!(addr & 0xFFF, 0);

        pt[pto] = addr; // Physical-address here!
        pt[pto] |= 0b01; // Present (P) Bit.
        pt[pto] |= 0b10; // Read/Write (R/W) Bit.
    }

    // Set CR3 to page-map level-4 table.
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    assert_eq!(kend & 0xFFF0000000000FFF, 0);

    states.set_cr3(kend); // Physical-address here!

    // Set CR4.
    let mut cr4 = 0;

    cr4 |= 0x20; // Physical-address extensions (PAE).

    states.set_cr4(cr4);

    // Set EFER.
    let mut efer = 0;

    efer |= 0x100; // Long Mode Enable (LME).
    efer |= 0x400; // Long Mode Active (LMA).

    states.set_efer(efer);

    // Set CR0.
    let mut cr0 = 0;

    cr0 |= 0x00000001; // Protected Mode Enable (PE).
    cr0 |= 0x80000000; // Paging (PG).

    states.set_cr0(cr0);

    // Set CS to 64-bit mode with ring 0. Although x86-64 specs from AMD ignore the Code/Data flag
    // on 64-bit mode but Intel CPU violate this spec so we need to enable it.
    states.set_cs(0b1000, 0, true, true, false);

    // Set data segments. The only fields used on 64-bit mode is P.
    states.set_ds(true);
    states.set_es(true);
    states.set_fs(true);
    states.set_gs(true);
    states.set_ss(true);

    // Set entry point, its argument and stack pointer.
    states.set_rdi(base + klen + slen);
    states.set_rsp(base + klen + slen); // Top-down.
    states.set_rip(base + entry);

    if let Err(e) = states.commit() {
        return Err(MainCpuError::CommitCpuStatesFailed(Box::new(e)));
    }

    // Check if PT_DYNAMIC valid.
    let &(p_vaddr, p_memsz) = dynamic;

    if p_memsz % 16 != 0 {
        return Err(MainCpuError::InvalidDynamicLinking);
    }

    match p_vaddr.checked_add(p_memsz) {
        Some(v) if v <= klen => {}
        _ => return Err(MainCpuError::InvalidDynamicLinking),
    }

    // Parse PT_DYNAMIC.
    let dynamic = std::slice::from_raw_parts(ram.host_addr().add(p_vaddr), p_memsz);
    let mut rela = None;
    let mut relasz = None;

    for entry in dynamic.chunks_exact(16) {
        let tag = usize::from_ne_bytes(entry[..8].try_into().unwrap());
        let val = usize::from_ne_bytes(entry[8..].try_into().unwrap());

        match tag {
            0 => break,              // DT_NULL
            7 => rela = Some(val),   // DT_RELA
            8 => relasz = Some(val), // DT_RELASZ
            _ => {}
        }
    }

    // Relocate the kernel to virtual address.
    let relocs = match (rela, relasz) {
        (None, None) => return Ok(()),
        (Some(rela), Some(relasz)) if relasz % 24 == 0 => match rela.checked_add(relasz) {
            Some(v) if v <= klen => std::slice::from_raw_parts(ram.host_addr().add(rela), relasz),
            _ => return Err(MainCpuError::InvalidDynamicLinking),
        },
        _ => return Err(MainCpuError::InvalidDynamicLinking),
    };

    for reloc in relocs.chunks_exact(24) {
        let r_offset = usize::from_ne_bytes(reloc[..8].try_into().unwrap());
        let r_info = usize::from_ne_bytes(reloc[8..16].try_into().unwrap());
        let r_addend = isize::from_ne_bytes(reloc[16..].try_into().unwrap());

        match r_info & 0xffffffff {
            // R_X86_64_NONE
            0 => break,
            // R_X86_64_RELATIVE
            8 => match r_offset.checked_add(8) {
                Some(v) if v <= klen => core::ptr::write_unaligned(
                    ram.host_addr().add(r_offset).cast_mut().cast(),
                    base.wrapping_add_signed(r_addend),
                ),
                _ => return Err(MainCpuError::InvalidDynamicLinking),
            },
            _ => {}
        }
    }

    Ok(())
}

/// # Safety
/// - The kernel must be mapped at address 0.
/// - The stack must be immediate follow the kernel.
#[cfg(target_arch = "aarch64")]
unsafe fn setup_main_cpu(
    cpu: &mut impl Cpu,
    ram: &Ram,
    klen: usize,
    slen: usize,
    entry: usize,
    dynamic: Option<&(usize, usize)>,
) -> Result<(), MainCpuError> {
    todo!()
}

#[cfg(target_arch = "x86_64")]
fn run_cpu(mut cpu: impl Cpu, args: &CpuArgs) {
    use self::hv::CpuIo;

    let mut logs = Vec::new();

    while !args.shutdown.load(Ordering::Relaxed) {
        // Run the vCPU and check why VM exit.
        let mut exit = cpu.run().unwrap();

        if let Some(io) = exit.is_io() {
            match io {
                CpuIo::Out(0, data) => {
                    logs.extend_from_slice(data);
                    parse_logs(&args.logs, &mut logs);
                }
                CpuIo::Out(_, _) => todo!(),
            }
        } else if !exit.is_hlt() {
            todo!()
        }
    }
}

#[cfg(target_arch = "aarch64")]
fn run_cpu(mut cpu: impl Cpu, args: &CpuArgs) {
    todo!()
}

#[cfg(target_arch = "x86_64")]
fn parse_logs(logs: &Mutex<VecDeque<(MsgType, String)>>, data: &mut Vec<u8>) {
    // Check minimum size.
    let (hdr, msg) = match data.split_at_checked(9) {
        Some(v) => v,
        None => return,
    };

    // Check if message completed.
    let len = usize::from_ne_bytes(hdr[1..].try_into().unwrap());
    let msg = match msg.get(..len) {
        Some(v) => v,
        None => return,
    };

    // Push to list.
    let ty = MsgType::from_u8(hdr[0]).unwrap();
    let msg = std::str::from_utf8(msg).unwrap().to_owned();
    let mut logs = logs.lock().unwrap();

    logs.push_back((ty, msg));

    while logs.len() > 10000 {
        logs.pop_front();
    }

    drop(logs);

    // Remove parsed data.
    data.drain(..(hdr.len() + len));
}

/// Manage a virtual machine that run the kernel.
pub struct Vmm {
    hv: Arc<self::hv::Default>,
    ram: Arc<Ram>,
    cpus: Vec<JoinHandle<()>>,
    screen: self::screen::Default,
    logs: Arc<Mutex<VecDeque<(MsgType, String)>>>,
    shutdown: Arc<AtomicBool>,
}

impl Drop for Vmm {
    fn drop(&mut self) {
        // Cancel all CPU threads.
        self.shutdown.store(true, Ordering::Relaxed);

        for cpu in self.cpus.drain(..) {
            cpu.join().unwrap();
        }
    }
}

/// Object that has a physical address in the virtual machine.
trait MemoryAddr {
    /// Physical address in the virtual machine.
    fn vm_addr(&self) -> usize;

    /// Address in our process.
    fn host_addr(&self) -> *const u8;

    /// Total size of the object, in bytes.
    fn len(&self) -> usize;
}

/// Encapsulates arguments for a function to run a CPU.
struct CpuArgs {
    hv: Arc<self::hv::Default>,
    screen: Arc<<self::screen::Default as Screen>::Buffer>,
    logs: Arc<Mutex<VecDeque<(MsgType, String)>>>,
    shutdown: Arc<AtomicBool>,
}

/// Represents an error when [`vmm_new()`] fails.
#[derive(Debug, Error)]
enum VmmError {
    #[error("couldn't get page size of the host")]
    GetPageSizeFailed(#[source] std::io::Error),

    #[error("host system is using an unsupported page size")]
    UnsupportedPageSize,

    #[error("couldn't create a RAM")]
    CreateRamFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(#[source] std::io::Error),

    #[error("your OS does not support 8 vCPU on a VM")]
    MaxCpuTooLow,

    #[cfg(target_os = "linux")]
    #[error("couldn't open /dev/kvm")]
    OpenKvmFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't get KVM version")]
    GetKvmVersionFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("unexpected KVM version")]
    KvmVersionMismatched,

    #[cfg(target_os = "linux")]
    #[error("couldn't create a VM")]
    CreateVmFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't map the RAM to the VM")]
    MapRamFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't get the size of vCPU mmap")]
    GetMmapSizeFailed(#[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't create WHP partition object ({0:#x})")]
    CreatePartitionFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "windows")]
    #[error("couldn't set number of CPU ({0:#x})")]
    SetCpuCountFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "windows")]
    #[error("couldn't setup WHP partition ({0:#x})")]
    SetupPartitionFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "windows")]
    #[error("couldn't map the RAM to WHP partition ({0:#x})")]
    MapRamFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "macos")]
    #[error("couldn't create a VM ({0:#x})")]
    CreateVmFailed(std::num::NonZero<std::ffi::c_int>),

    #[cfg(target_os = "macos")]
    #[error("couldn't map memory to the VM")]
    MapRamFailed(std::num::NonZero<std::ffi::c_int>),

    #[cfg(target_os = "macos")]
    #[error("couldn't get default MTLDevice")]
    GetMetalDeviceFailed,
}

/// Represents an error when [`main_cpu()`] fails to reach event loop.
#[derive(Debug, Error)]
enum MainCpuError {
    #[error("couldn't create vCPU")]
    CreateCpuFailed(#[source] <self::hv::Default as Hypervisor>::CpuErr),

    #[error("couldn't allocate RAM for kernel arguments")]
    AllocArgsFailed(#[source] std::io::Error),

    #[cfg(target_arch = "x86_64")]
    #[error("the kernel is not a position-independent executable")]
    NonPieKernel,

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate RAM for page-map level-4 table")]
    AllocPml4TableFailed(#[source] std::io::Error),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate RAM for page-directory pointer table")]
    AllocPdpTableFailed(#[source] std::io::Error),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate RAM for page-directory table")]
    AllocPdTableFailed(#[source] std::io::Error),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate RAM for page table")]
    AllocPageTableFailed(#[source] std::io::Error),

    #[cfg(target_arch = "x86_64")]
    #[error("the kernel has invalid PT_DYNAMIC")]
    InvalidDynamicLinking,

    #[error("couldn't get vCPU states")]
    GetCpuStatesFailed(#[source] Box<dyn Error + Send>),

    #[error("couldn't commit vCPU states")]
    CommitCpuStatesFailed(#[source] Box<dyn Error + Send>),
}
