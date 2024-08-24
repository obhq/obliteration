use self::hv::{Cpu, CpuExit, CpuIo, CpuStates, Hypervisor};
use self::hw::{setup_devices, Device, DeviceContext, DeviceTree, Ram, RamBuilder, RamMap};
use self::kernel::Kernel;
use self::screen::Screen;
use crate::error::RustError;
use obconf::{BootEnv, Vm};
use obvirt::console::MsgType;
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::{c_char, c_void, CStr};
use std::io::Read;
use std::num::NonZero;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use thiserror::Error;

mod hv;
mod hw;
mod kernel;
mod screen;

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_run(
    kernel: *const c_char,
    screen: *const VmmScreen,
    event: unsafe extern "C" fn(*const VmmEvent, *mut c_void) -> bool,
    cx: *mut c_void,
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
    let mut file = match Kernel::open(path) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't open {path}"), e);
            return null_mut();
        }
    };

    // Get program header enumerator.
    let hdrs = match file.program_headers() {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(
                format_args!("couldn't start enumerating program headers of {path}"),
                e,
            );

            return null_mut();
        }
    };

    // Parse program headers.
    let mut segments = Vec::new();
    let mut dynamic = None;
    let mut note = None;

    for (index, item) in hdrs.enumerate() {
        // Check if success.
        let hdr = match item {
            Ok(v) => v,
            Err(e) => {
                *err = RustError::with_source(
                    format_args!("couldn't read program header #{index} on {path}"),
                    e,
                );

                return null_mut();
            }
        };

        // Process the header.
        match hdr.p_type {
            1 => {
                if hdr.p_filesz > TryInto::<u64>::try_into(hdr.p_memsz).unwrap() {
                    *err = RustError::new(format!("invalid p_filesz on on PT_LOAD {index}"));
                    return null_mut();
                }

                segments.push(hdr);
            }
            2 => {
                if dynamic.is_some() {
                    *err = RustError::new("multiple PT_DYNAMIC is not supported");
                    return null_mut();
                }

                dynamic = Some(hdr);
            }
            4 => {
                if note.is_some() {
                    *err = RustError::new("multiple PT_NOTE is not supported");
                    return null_mut();
                }

                note = Some(hdr);
            }
            6 | 1685382481 | 1685382482 => {}
            v => {
                *err = RustError::new(format!("unknown p_type {v} on program header {index}"));
                return null_mut();
            }
        }
    }

    segments.sort_unstable_by_key(|i| i.p_vaddr);

    // Make sure the first PT_LOAD includes the ELF header.
    match segments.first() {
        Some(hdr) => {
            if hdr.p_offset != 0 {
                *err = RustError::new("the first PT_LOAD does not includes ELF header");
                return null_mut();
            }
        }
        None => {
            *err = RustError::new("no any PT_LOAD on the kernel");
            return null_mut();
        }
    }

    // Check if PT_NOTE exists.
    let note = match note {
        Some(v) => v,
        None => {
            *err = RustError::new("no PT_NOTE segment on the kernel");
            return null_mut();
        }
    };

    // Seek to PT_NOTE.
    let mut data = match file.segment_data(&note) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't seek to PT_NOTE on {path}"), e);
            return null_mut();
        }
    };

    // Parse PT_NOTE.
    let mut vm_page_size = None;

    for i in 0.. {
        // Check remaining data.
        if data.limit() == 0 {
            break;
        }

        // Read note header.
        let mut buf = [0u8; 4 * 3];

        if let Err(e) = data.read_exact(&mut buf) {
            *err = RustError::with_source(format_args!("couldn't read kernel note #{i} header"), e);
            return null_mut();
        }

        // Parse note header.
        let nlen: usize = u32::from_ne_bytes(buf[..4].try_into().unwrap())
            .try_into()
            .unwrap();
        let dlen: usize = u32::from_ne_bytes(buf[4..8].try_into().unwrap())
            .try_into()
            .unwrap();
        let ty = u32::from_ne_bytes(buf[8..].try_into().unwrap());

        if nlen > 0xff {
            *err = RustError::new(format!("name on kernel note #{i} is too large"));
            return null_mut();
        }

        if dlen > 0xff {
            *err = RustError::new(format!("description on kernel note #{i} is too large"));
            return null_mut();
        }

        // Read note name + description.
        let nalign = nlen.next_multiple_of(4);
        let mut buf = vec![0u8; nalign + dlen];

        if let Err(e) = data.read_exact(&mut buf) {
            *err = RustError::with_source(format_args!("couldn't read kernel note #{i} data"), e);
            return null_mut();
        }

        // Check name.
        let name = match CStr::from_bytes_until_nul(&buf) {
            Ok(v) if v.to_bytes_with_nul().len() == nlen => v,
            _ => {
                *err = RustError::new(format!("kernel note #{i} has invalid name"));
                return null_mut();
            }
        };

        if name.to_bytes() != b"obkrnl" {
            continue;
        }

        // Parse description.
        match ty {
            0 => {
                if vm_page_size.is_some() {
                    *err = RustError::new(format!("kernel note #{i} is duplicated"));
                    return null_mut();
                }

                vm_page_size = buf[nalign..]
                    .try_into()
                    .map(usize::from_ne_bytes)
                    .ok()
                    .and_then(NonZero::new)
                    .filter(|v| v.is_power_of_two());

                if vm_page_size.is_none() {
                    *err = RustError::new(format!("invalid description on kernel note #{i}"));
                    return null_mut();
                }
            }
            v => {
                *err = RustError::new(format!("unknown type {v} on kernel note #{i}"));
                return null_mut();
            }
        }
    }

    // Check if page size exists.
    let vm_page_size = match vm_page_size {
        Some(v) => v,
        None => {
            *err = RustError::new("no page size in kernel note");
            return null_mut();
        }
    };

    // TODO: Support any page size on the host. With page size the same as kernel or lower we don't
    // need to keep track allocations in the RAM because any requested address from the kernel will
    // always page-aligned on the host.
    let host_page_size = match get_page_size() {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't get host page size", e);
            return null_mut();
        }
    };

    if host_page_size > vm_page_size {
        *err = RustError::new("your system using an unsupported page size");
        return null_mut();
    }

    // Get kernel memory size.
    let mut len = 0;

    for hdr in &segments {
        if hdr.p_vaddr < len {
            *err = RustError::new(format!(
                "PT_LOAD at {:#x} is overlapped with the previous PT_LOAD",
                hdr.p_vaddr
            ));

            return null_mut();
        }

        len = match hdr.p_vaddr.checked_add(hdr.p_memsz) {
            Some(v) => v,
            None => {
                *err = RustError::new(format!("invalid p_memsz on PT_LOAD at {:#x}", hdr.p_vaddr));
                return null_mut();
            }
        };
    }

    // Round kernel memory size.
    let len = match len {
        0 => {
            *err = RustError::new("the kernel has PT_LOAD with zero length");
            return null_mut();
        }
        v => match v.checked_next_multiple_of(vm_page_size.get()) {
            Some(v) => NonZero::new_unchecked(v),
            None => {
                *err = RustError::new("total size of PT_LOAD is too large");
                return null_mut();
            }
        },
    };

    // Setup RAM builder.
    let mut ram = match RamBuilder::new(vm_page_size) {
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

    for hdr in &segments {
        // Seek to segment data.
        let mut data = match file.segment_data(hdr) {
            Ok(v) => v,
            Err(e) => {
                *err = RustError::with_source(
                    format_args!("couldn't seek to offset {}", hdr.p_offset),
                    e,
                );

                return null_mut();
            }
        };

        // Read segment data.
        let mut seg = &mut kern[hdr.p_vaddr..(hdr.p_vaddr + hdr.p_memsz)];

        match std::io::copy(&mut data, &mut seg) {
            Ok(v) => {
                if v != hdr.p_filesz {
                    *err = RustError::new(format!("{path} is incomplete"));
                    return null_mut();
                }
            }
            Err(e) => {
                *err = RustError::with_source(
                    format_args!("couldn't read kernet at offset {}", hdr.p_offset),
                    e,
                );

                return null_mut();
            }
        }
    }

    // Allocate stack.
    if let Err(e) = ram.alloc_stack(NonZero::new(1024 * 1024 * 2).unwrap()) {
        *err = RustError::with_source("couldn't allocate RAM for stack", e);
        return null_mut();
    }

    // Allocate arguments.
    let event = VmmEventHandler { fp: event, cx };
    let devices = Arc::new(setup_devices(Ram::SIZE, vm_page_size, event));
    let env = BootEnv::Vm(Vm {
        console: devices.console().addr(),
    });

    if let Err(e) = ram.alloc_args(env) {
        *err = RustError::with_source("couldn't allocate RAM for arguments", e);
        return null_mut();
    }

    // Build RAM.
    let (ram, map) = match ram.build(&devices, dynamic) {
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
    let shutdown = Arc::new(AtomicBool::new(false));
    let args = CpuArgs {
        hv,
        ram,
        screen: screen.buffer().clone(),
        devices,
        shutdown: shutdown.clone(),
    };

    // Spawn a thread to drive main CPU.
    let e_entry = file.entry();
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
        cpus: vec![main],
        screen,
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
    // Enable MMU to enable virtual address.
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    states.set_sctlr_el1(true);

    // Uses 48-bit Intermediate Physical Address (ips = 0b101) and 48-bit virtual addresses
    // (t?sz = 16) for both kernel space and user space. Use ASID from user space (a1 = 0).
    states.set_tcr_el1(
        0b101,
        match map.page_size.get() {
            0x4000 => 0b01,
            _ => todo!(),
        },
        false,
        16,
        match map.page_size.get() {
            0x4000 => 0b10,
            _ => todo!(),
        },
        16,
    );

    // Set page table. We need both lower and higher VA here because the virtual devices mapped with
    // identity mapping.
    states.set_ttbr0_el1(map.page_table);
    states.set_ttbr1_el1(map.page_table);

    // Set entry point, its argument and stack pointer.
    states.set_x0(map.env_vaddr);
    states.set_sp_el1(map.stack_vaddr + map.stack_len); // Top-down.
    states.set_pc(map.kern_vaddr + entry);

    states
        .commit()
        .map_err(|e| MainCpuError::CommitCpuStatesFailed(Box::new(e)))
}

fn run_cpu(mut cpu: impl Cpu, args: &CpuArgs) {
    let mut devices = args
        .devices
        .map()
        .map(|(addr, dev)| {
            let end = dev.len().checked_add(addr).unwrap();

            (addr, (dev.create_context(&args.ram), end))
        })
        .collect::<BTreeMap<usize, (Box<dyn DeviceContext>, NonZero<usize>)>>();

    while !args.shutdown.load(Ordering::Relaxed) {
        // Run the vCPU.
        let exit = match cpu.run() {
            Ok(v) => v,
            Err(_) => todo!(),
        };

        // Check if HLT.
        #[cfg(target_arch = "x86_64")]
        let exit = match exit.into_hlt() {
            Ok(_) => continue,
            Err(v) => v,
        };

        // Check if I/O.
        match exit.into_io() {
            Ok(io) => match exec_io(&mut devices, io) {
                Ok(status) => {
                    if !status {
                        args.shutdown.store(true, Ordering::Relaxed);
                    }

                    continue;
                }
                Err(_) => todo!(),
            },
            Err(_) => todo!(),
        }
    }
}

fn exec_io<'a>(
    devices: &mut BTreeMap<usize, (Box<dyn DeviceContext + 'a>, NonZero<usize>)>,
    mut io: impl CpuIo,
) -> Result<bool, Box<dyn Error>> {
    // Get target device.
    let addr = io.addr();
    let (_, (dev, end)) = devices.range_mut(..=addr).last().unwrap();

    assert!(addr < end.get());

    dev.exec(&mut io)
}

#[cfg(unix)]
fn get_page_size() -> Result<NonZero<usize>, std::io::Error> {
    let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

    if v < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(v.try_into().ok().and_then(NonZero::new).unwrap())
    }
}

#[cfg(windows)]
fn get_page_size() -> Result<NonZero<usize>, std::io::Error> {
    use std::mem::zeroed;
    use windows_sys::Win32::System::SystemInformation::GetSystemInfo;
    let mut i = unsafe { zeroed() };

    unsafe { GetSystemInfo(&mut i) };

    Ok(i.dwPageSize.try_into().ok().and_then(NonZero::new).unwrap())
}

/// Manage a virtual machine that run the kernel.
pub struct Vmm {
    cpus: Vec<JoinHandle<()>>,
    screen: self::screen::Default,
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

/// Contains objects required to render the screen.
#[repr(C)]
pub struct VmmScreen {
    #[cfg(not(target_os = "macos"))]
    pub vk_instance: usize,
    #[cfg(not(target_os = "macos"))]
    pub vk_surface: usize,
    #[cfg(target_os = "macos")]
    pub view: usize,
}

/// Encapsulates a function to handle VMM events.
#[derive(Clone, Copy)]
struct VmmEventHandler {
    fp: unsafe extern "C" fn(*const VmmEvent, *mut c_void) -> bool,
    cx: *mut c_void,
}

impl VmmEventHandler {
    unsafe fn invoke(self, e: VmmEvent) -> bool {
        (self.fp)(&e, self.cx)
    }
}

unsafe impl Send for VmmEventHandler {}
unsafe impl Sync for VmmEventHandler {}

/// Contains VMM event information.
#[repr(C)]
#[allow(dead_code)] // TODO: Figure out why Rust think fields in each enum are not used.
pub enum VmmEvent {
    Log {
        ty: VmmLog,
        data: *const c_char,
        len: usize,
    },
}

/// Log category.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum VmmLog {
    Info,
}

impl From<MsgType> for VmmLog {
    fn from(value: MsgType) -> Self {
        match value {
            MsgType::Info => Self::Info,
        }
    }
}

/// Encapsulates arguments for a function to run a CPU.
struct CpuArgs {
    hv: Arc<self::hv::Default>,
    ram: Arc<Ram>,
    screen: Arc<<self::screen::Default as Screen>::Buffer>,
    devices: Arc<DeviceTree>,
    shutdown: Arc<AtomicBool>,
}

/// Represents an error when [`vmm_new()`] fails.
#[derive(Debug, Error)]
enum VmmError {
    #[error("couldn't create a RAM")]
    CreateRamFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(#[source] std::io::Error),

    #[cfg(not(target_os = "macos"))]
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
