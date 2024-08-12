use self::hv::{Cpu, CpuExit, CpuStates, Hypervisor};
use self::hw::{Ram, RamBuilder, RamMap};
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
mod hw;
mod screen;

#[cfg(target_arch = "x86_64")]
const ELF_MACHINE: u16 = 62;
#[cfg(target_arch = "aarch64")]
const ELF_MACHINE: u16 = 183;

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_run(
    kernel: *const c_char,
    screen: usize,
    err: *mut *mut RustError,
) -> *mut Vmm {
    // Check if path UTF-8.
    let path = match CStr::from_ptr(kernel).to_str() {
        Ok(v) => v,
        Err(_) => {
            *err = RustError::new("path of the kernel is not UTF-8");
            return null_mut();
        }
    };

    // Open kernel image.
    let mut file = match File::open(path) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't open the kernel", e);
            return null_mut();
        }
    };

    // Read file header.
    let mut hdr = [0; 64];

    if let Err(e) = file.read_exact(&mut hdr) {
        *err = RustError::with_source("couldn't read kernel header", e);
        return null_mut();
    }

    // Check if ELF.
    if &hdr[..4] != b"\x7fELF" {
        *err = RustError::new("the kernel is not an ELF file");
        return null_mut();
    }

    // Check ELF type.
    if hdr[4] != 2 {
        *err = RustError::new("the kernel is not 64-bit kernel");
        return null_mut();
    }

    if hdr[6] != 1 {
        *err = RustError::new("the kernel has unknown ELF version");
        return null_mut();
    }

    if u16::from_ne_bytes(hdr[18..20].try_into().unwrap()) != ELF_MACHINE {
        *err = RustError::new("the kernel is for a different CPU architecture");
        return null_mut();
    }

    // Load ELF header.
    let e_entry = usize::from_ne_bytes(hdr[24..32].try_into().unwrap());
    let e_phoff = u64::from_ne_bytes(hdr[32..40].try_into().unwrap());
    let e_phentsize: usize = u16::from_ne_bytes(hdr[54..56].try_into().unwrap()).into();
    let e_phnum: usize = u16::from_ne_bytes(hdr[56..58].try_into().unwrap()).into();

    if e_phentsize != 56 {
        *err = RustError::new("the kernel has unsupported e_phentsize");
        return null_mut();
    }

    if e_phnum > 16 {
        *err = RustError::new("too many program headers");
        return null_mut();
    }

    // Seek to program headers.
    match file.seek(SeekFrom::Start(e_phoff)) {
        Ok(v) => {
            if v != e_phoff {
                *err = RustError::new("the kernel is incomplete");
                return null_mut();
            }
        }
        Err(e) => {
            *err = RustError::with_source("couldn't seek to program headers", e);
            return null_mut();
        }
    }

    // Read program headers.
    let mut data = vec![0; e_phnum * e_phentsize];

    if let Err(e) = file.read_exact(&mut data) {
        *err = RustError::with_source("couldn't read program headers", e);
        return null_mut();
    }

    // Parse program headers.
    let mut segments = Vec::with_capacity(e_phnum);
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
                    *err = RustError::new(format!("invalid p_filesz on on PT_LOAD {index}"));
                    return null_mut();
                }

                if p_align != Ram::VM_PAGE_SIZE {
                    *err = RustError::new(format!("unsupported p_align on PT_LOAD {index}"));
                    return null_mut();
                }

                segments.push((p_offset, p_filesz, p_vaddr, p_memsz));
            }
            2 => {
                if dynamic.is_some() {
                    *err = RustError::new("multiple PT_DYNAMIC is not supported");
                    return null_mut();
                }

                dynamic = Some((p_vaddr, p_memsz));
            }
            6 | 1685382481 | 1685382482 => {}
            v => {
                *err = RustError::new(format!("unknown p_type {v} on program header {index}"));
                return null_mut();
            }
        }
    }

    segments.sort_unstable_by_key(|i| i.2);

    // Make sure the first PT_LOAD includes the ELF header.
    match segments.first() {
        Some(&(p_offset, _, _, _)) => {
            if p_offset != 0 {
                *err = RustError::new("the first PT_LOAD does not includes ELF header");
                return null_mut();
            }
        }
        None => {
            *err = RustError::new("no any PT_LOAD on the kernel");
            return null_mut();
        }
    }

    // Get kernel memory size.
    let mut len = 0;

    for &(_, _, p_vaddr, p_memsz) in &segments {
        if p_vaddr < len {
            *err = RustError::new(format!(
                "PT_LOAD at {p_vaddr:#x} is overlapped with the previous PT_LOAD"
            ));
            return null_mut();
        }

        len = match p_vaddr
            .checked_add(p_memsz)
            .and_then(|end| end.checked_next_multiple_of(Ram::VM_PAGE_SIZE))
        {
            Some(v) => v,
            None => {
                *err = RustError::new(format!("invalid p_memsz on PT_LOAD at {p_vaddr:#x}"));
                return null_mut();
            }
        };
    }

    // Setup RAM builder.
    let mut ram = match RamBuilder::new() {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Map the kernel.
    let kern = match ram.alloc_kernel(len) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't allocate RAM for the kernel", e);
            return null_mut();
        }
    };

    for &(p_offset, p_filesz, p_vaddr, p_memsz) in &segments {
        // Seek to segment data.
        match file.seek(SeekFrom::Start(p_offset)) {
            Ok(v) => {
                if v != p_offset {
                    *err = RustError::new("the kernel is incomplete");
                    return null_mut();
                }
            }
            Err(e) => {
                *err = RustError::with_source(format!("couldn't seek to offset {p_offset}"), e);
                return null_mut();
            }
        }

        // Read segment data.
        let seg = &mut kern[p_vaddr..(p_vaddr + p_memsz)];

        if let Err(e) = file.read_exact(&mut seg[..p_filesz]) {
            *err = RustError::with_source(format!("couldn't read kernet at offset {p_offset}"), e);
            return null_mut();
        }
    }

    // Allocate stack.
    if let Err(e) = ram.alloc_stack(1024 * 1024 * 2) {
        *err = RustError::with_source("couldn't allocate RAM for stack", e);
        return null_mut();
    }

    // Allocate arguments.
    let env = BootEnv::Vm(Vm {});

    if let Err(e) = ram.alloc_args(env) {
        *err = RustError::with_source("couldn't allocate RAM for arguments", e);
        return null_mut();
    }

    // Build RAM.
    let (ram, map) = match ram.build(dynamic) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't build RAM", e);
            return null_mut();
        }
    };

    // Setup hypervisor.
    let ram = Arc::new(ram);
    let hv = match self::hv::Default::new(8, ram.clone()) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            *err = RustError::with_source("couldn't setup a hypervisor", e);
            return null_mut();
        }
    };

    // Setup screen.
    let screen = match self::screen::Default::new(screen) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't setup a screen", e);
            return null_mut();
        }
    };

    // Setup arguments for main CPU.
    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let shutdown = Arc::new(AtomicBool::new(false));
    let args = CpuArgs {
        hv: hv.clone(),
        screen: screen.buffer().clone(),
        logs: logs.clone(),
        shutdown: shutdown.clone(),
    };

    // Spawn a thread to drive main CPU.
    let (tx, rx) = std::sync::mpsc::channel();
    let main = move || main_cpu(&args, e_entry, map, tx);
    let main = match std::thread::Builder::new().spawn(main) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't spawn main CPU", e);
            return null_mut();
        }
    };

    // Wait for main CPU to enter event loop.
    let r = match rx.recv() {
        Ok(v) => v,
        Err(_) => {
            main.join().unwrap();
            *err = RustError::new("main CPU stopped unexpectedly");
            return null_mut();
        }
    };

    if let Err(e) = r {
        main.join().unwrap();
        *err = RustError::with_source("couldn't start main CPU", e);
        return null_mut();
    }

    // Create VMM.
    let vmm = Vmm {
        hv,
        ram,
        cpus: vec![main],
        screen,
        logs,
        shutdown,
    };

    Box::into_raw(vmm.into())
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

fn main_cpu(args: &CpuArgs, entry: usize, map: RamMap, status: Sender<Result<(), MainCpuError>>) {
    // Create vCPU.
    let mut cpu = match args.hv.create_cpu(0) {
        Ok(v) => v,
        Err(e) => {
            status.send(Err(MainCpuError::CreateCpuFailed(e))).unwrap();
            return;
        }
    };

    if let Err(e) = setup_main_cpu(&mut cpu, entry, map) {
        status.send(Err(e)).unwrap();
        return;
    }

    // Enter dispatch loop.
    status.send(Ok(())).unwrap();
    drop(status);

    run_cpu(cpu, args);
}

#[cfg(target_arch = "x86_64")]
fn setup_main_cpu(cpu: &mut impl Cpu, entry: usize, map: RamMap) -> Result<(), MainCpuError> {
    // Set CR3 to page-map level-4 table.
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    assert_eq!(map.page_table & 0xFFF0000000000FFF, 0);

    states.set_cr3(map.page_table);

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
    states.set_rdi(map.env_vaddr);
    states.set_rsp(map.stack_vaddr + map.stack_len); // Top-down.
    states.set_rip(map.kern_vaddr + entry);

    if let Err(e) = states.commit() {
        return Err(MainCpuError::CommitCpuStatesFailed(Box::new(e)));
    }

    Ok(())
}

#[cfg(target_arch = "aarch64")]
fn setup_main_cpu(cpu: &mut impl Cpu, entry: usize, map: RamMap) -> Result<(), MainCpuError> {
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

    #[error("couldn't get vCPU states")]
    GetCpuStatesFailed(#[source] Box<dyn Error + Send>),

    #[error("couldn't commit vCPU states")]
    CommitCpuStatesFailed(#[source] Box<dyn Error + Send>),
}
