// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::CpuManager;
use self::hv::Hypervisor;
use self::hw::{setup_devices, Device};
use self::kernel::{
    Kernel, PT_DYNAMIC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_STACK, PT_LOAD, PT_NOTE, PT_PHDR,
};
use self::ram::{Ram, RamBuilder};
use self::screen::Screen;
use crate::debug::DebugClient;
use crate::error::RustError;
use crate::profile::Profile;
use gdbstub::common::Signal;
use gdbstub::stub::state_machine::GdbStubStateMachine;
use gdbstub::stub::MultiThreadStopReason;
use obconf::{BootEnv, ConsoleType, Vm};
use std::cmp::max;
use std::error::Error;
use std::ffi::{c_char, c_void, CStr};
use std::io::Read;
use std::num::NonZero;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod debug;
mod hv;
mod hw;
mod kernel;
mod ram;
mod screen;

#[no_mangle]
pub unsafe extern "C" fn vmm_start(
    kernel: *const c_char,
    screen: *const VmmScreen,
    profile: *const Profile,
    debugger: *mut DebugClient,
    event: unsafe extern "C" fn(*const VmmEvent, *mut c_void),
    cx: *mut c_void,
    err: *mut *mut RustError,
) -> *mut Vmm {
    // Consume the debugger now to prevent memory leak in case of error.
    let debugger = match debugger.is_null() {
        true => None,
        false => Some(Box::from_raw(debugger)),
    };

    // Check if path UTF-8.
    let path = match CStr::from_ptr(kernel).to_str() {
        Ok(v) => v,
        Err(_) => {
            *err = RustError::new("path of the kernel is not UTF-8").into_c();
            return null_mut();
        }
    };

    // Open kernel image.
    let mut file = match Kernel::open(path) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't open {path}"), e).into_c();
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
            )
            .into_c();

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
                )
                .into_c();

                return null_mut();
            }
        };

        // Process the header.
        match hdr.p_type {
            PT_LOAD => {
                if hdr.p_filesz > TryInto::<u64>::try_into(hdr.p_memsz).unwrap() {
                    *err =
                        RustError::new(format!("invalid p_filesz on on PT_LOAD {index}")).into_c();
                    return null_mut();
                }

                segments.push(hdr);
            }
            PT_DYNAMIC => {
                if dynamic.is_some() {
                    *err = RustError::new("multiple PT_DYNAMIC is not supported").into_c();
                    return null_mut();
                }

                dynamic = Some(hdr);
            }
            PT_NOTE => {
                if note.is_some() {
                    *err = RustError::new("multiple PT_NOTE is not supported").into_c();
                    return null_mut();
                }

                note = Some(hdr);
            }
            PT_PHDR | PT_GNU_EH_FRAME | PT_GNU_STACK | PT_GNU_RELRO => {}
            v => {
                *err = RustError::new(format!("unknown p_type {v} on program header {index}"))
                    .into_c();
                return null_mut();
            }
        }
    }

    segments.sort_unstable_by_key(|i| i.p_vaddr);

    // Make sure the first PT_LOAD includes the ELF header.
    match segments.first() {
        Some(hdr) => {
            if hdr.p_offset != 0 {
                *err = RustError::new("the first PT_LOAD does not includes ELF header").into_c();
                return null_mut();
            }
        }
        None => {
            *err = RustError::new("no any PT_LOAD on the kernel").into_c();
            return null_mut();
        }
    }

    // Check if PT_DYNAMIC exists.
    let dynamic = match dynamic {
        Some(v) => v,
        None => {
            *err = RustError::new("no PT_DYNAMIC segment on the kernel").into_c();
            return null_mut();
        }
    };

    // Check if PT_NOTE exists.
    let note = match note {
        Some(v) => v,
        None => {
            *err = RustError::new("no PT_NOTE segment on the kernel").into_c();
            return null_mut();
        }
    };

    // Seek to PT_NOTE.
    let mut data = match file.segment_data(&note) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't seek to PT_NOTE on {path}"), e)
                .into_c();
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
            *err = RustError::with_source(format_args!("couldn't read kernel note #{i} header"), e)
                .into_c();
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
            *err = RustError::new(format!("name on kernel note #{i} is too large")).into_c();
            return null_mut();
        }

        if dlen > 0xff {
            *err = RustError::new(format!("description on kernel note #{i} is too large")).into_c();
            return null_mut();
        }

        // Read note name + description.
        let nalign = nlen.next_multiple_of(4);
        let mut buf = vec![0u8; nalign + dlen];

        if let Err(e) = data.read_exact(&mut buf) {
            *err = RustError::with_source(format_args!("couldn't read kernel note #{i} data"), e)
                .into_c();
            return null_mut();
        }

        // Check name.
        let name = match CStr::from_bytes_until_nul(&buf) {
            Ok(v) if v.to_bytes_with_nul().len() == nlen => v,
            _ => {
                *err = RustError::new(format!("kernel note #{i} has invalid name")).into_c();
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
                    *err = RustError::new(format!("kernel note #{i} is duplicated")).into_c();
                    return null_mut();
                }

                vm_page_size = buf[nalign..]
                    .try_into()
                    .map(usize::from_ne_bytes)
                    .ok()
                    .and_then(NonZero::new)
                    .filter(|v| v.is_power_of_two());

                if vm_page_size.is_none() {
                    *err =
                        RustError::new(format!("invalid description on kernel note #{i}")).into_c();
                    return null_mut();
                }
            }
            v => {
                *err = RustError::new(format!("unknown type {v} on kernel note #{i}")).into_c();
                return null_mut();
            }
        }
    }

    // Check if page size exists.
    let vm_page_size = match vm_page_size {
        Some(v) => v,
        None => {
            *err = RustError::new("no page size in kernel note").into_c();
            return null_mut();
        }
    };

    // Get page size on the host.
    let host_page_size = match get_page_size() {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't get host page size", e).into_c();
            return null_mut();
        }
    };

    // Get kernel memory size.
    let mut len = 0;

    for hdr in &segments {
        if hdr.p_vaddr < len {
            *err = RustError::new(format!(
                "PT_LOAD at {:#x} is overlapped with the previous PT_LOAD",
                hdr.p_vaddr
            ))
            .into_c();

            return null_mut();
        }

        len = match hdr.p_vaddr.checked_add(hdr.p_memsz) {
            Some(v) => v,
            None => {
                *err = RustError::new(format!("invalid p_memsz on PT_LOAD at {:#x}", hdr.p_vaddr))
                    .into_c();
                return null_mut();
            }
        };
    }

    // Round kernel memory size.
    let block_size = max(vm_page_size, host_page_size);
    let len = match len {
        0 => {
            *err = RustError::new("the kernel has PT_LOAD with zero length").into_c();
            return null_mut();
        }
        v => match v.checked_next_multiple_of(block_size.get()) {
            Some(v) => NonZero::new_unchecked(v),
            None => {
                *err = RustError::new("total size of PT_LOAD is too large").into_c();
                return null_mut();
            }
        },
    };

    // Setup RAM.
    let ram = match Ram::new(NonZero::new(1024 * 1024 * 1024 * 8).unwrap(), block_size) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't create a RAM", e).into_c();
            return null_mut();
        }
    };

    // Setup virtual devices.
    let event = VmmEventHandler { fp: event, cx };
    let devices = Arc::new(setup_devices(ram.len().get(), block_size, event));

    // Setup hypervisor.
    let mut hv = match self::hv::new(8, ram, debugger.is_some()) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't setup a hypervisor", e).into_c();
            return null_mut();
        }
    };

    // Map the kernel.
    let feats = hv.cpu_features().clone();
    let mut ram = RamBuilder::new(hv.ram_mut());
    let kern = match ram.alloc_kernel(len) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't allocate RAM for the kernel", e).into_c();
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
                )
                .into_c();

                return null_mut();
            }
        };

        // Read segment data.
        let mut seg = &mut kern[hdr.p_vaddr..(hdr.p_vaddr + hdr.p_memsz)];

        match std::io::copy(&mut data, &mut seg) {
            Ok(v) => {
                if v != hdr.p_filesz {
                    *err = RustError::new(format!("{path} is incomplete")).into_c();
                    return null_mut();
                }
            }
            Err(e) => {
                *err = RustError::with_source(
                    format_args!("couldn't read kernet at offset {}", hdr.p_offset),
                    e,
                )
                .into_c();

                return null_mut();
            }
        }
    }

    // Allocate stack.
    if let Err(e) = ram.alloc_stack(NonZero::new(1024 * 1024 * 2).unwrap()) {
        *err = RustError::with_source("couldn't allocate RAM for stack", e).into_c();
        return null_mut();
    }

    // Allocate arguments.
    let env = BootEnv::Vm(Vm {
        vmm: devices.vmm().addr(),
        console: devices.console().addr(),
        host_page_size,
    });

    if let Err(e) = ram.alloc_args(env, (*profile).kernel_config().clone()) {
        *err = RustError::with_source("couldn't allocate RAM for arguments", e).into_c();
        return null_mut();
    }

    // Build RAM.
    let map = match ram.build(&feats, vm_page_size, &devices, dynamic) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't build RAM", e).into_c();
            return null_mut();
        }
    };

    // Setup screen.
    let screen = match self::screen::Default::new(&*screen) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't setup a screen", e).into_c();
            return null_mut();
        }
    };

    // Setup CPU manager.
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut cpu = CpuManager::new(
        Arc::new(hv),
        screen.buffer().clone(),
        devices,
        event,
        shutdown.clone(),
    );

    // Setup GDB stub.
    let gdb = match debugger
        .map(|client| {
            gdbstub::stub::GdbStub::new(*client)
                .run_state_machine(&mut cpu)
                .map_err(|e| RustError::with_source("couldn't setup a GDB stub", e))
        })
        .transpose()
    {
        Ok(v) => v,
        Err(e) => {
            *err = e.into_c();
            return null_mut();
        }
    };

    // Spawn main CPU.
    cpu.spawn(map.kern_vaddr + file.entry(), Some(map), gdb.is_some());

    // Create VMM.
    let vmm = Vmm {
        cpu,
        screen,
        gdb,
        shutdown,
    };

    Box::into_raw(vmm.into())
}

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_draw(vmm: *mut Vmm) -> *mut RustError {
    match (*vmm).screen.update() {
        Ok(_) => null_mut(),
        Err(e) => RustError::wrap(e).into_c(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_dispatch_debug(vmm: *mut Vmm, stop: *mut KernelStop) -> DebugResult {
    // Consume stop reason now to prevent memory leak.
    let vmm = &mut *vmm;
    let mut stop = match stop.is_null() {
        true => None,
        false => Some(Box::from_raw(stop).0),
    };

    loop {
        // Check current state.
        let r = match vmm.gdb.take().unwrap() {
            GdbStubStateMachine::Idle(s) => match self::debug::dispatch_idle(&mut vmm.cpu, s) {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(v)) => {
                    // No pending data from the debugger.
                    vmm.gdb = Some(v.into());
                    return DebugResult::Ok;
                }
                Err(e) => Err(e),
            },
            GdbStubStateMachine::Running(s) => {
                match self::debug::dispatch_running(&mut vmm.cpu, s, stop.take()) {
                    Ok(Ok(v)) => Ok(v),
                    Ok(Err(v)) => {
                        // No pending data from the debugger.
                        vmm.gdb = Some(v.into());
                        vmm.cpu.release();
                        return DebugResult::Ok;
                    }
                    Err(e) => Err(e),
                }
            }
            GdbStubStateMachine::CtrlCInterrupt(s) => {
                vmm.cpu.lock();

                s.interrupt_handled(
                    &mut vmm.cpu,
                    Some(MultiThreadStopReason::Signal(Signal::SIGINT)),
                )
                .map_err(|e| RustError::with_source("couldn't handle CTRL+C from a debugger", e))
            }
            GdbStubStateMachine::Disconnected(_) => return DebugResult::Disconnected,
        };

        match r {
            Ok(v) => vmm.gdb = Some(v),
            Err(e) => return DebugResult::Error { reason: e.into_c() },
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_debug_socket(vmm: *mut Vmm) -> isize {
    let s = match &mut (*vmm).gdb {
        Some(v) => v,
        None => return -1,
    };

    match s {
        GdbStubStateMachine::Idle(s) => s.borrow_conn().socket() as _,
        GdbStubStateMachine::Running(s) => s.borrow_conn().socket() as _,
        GdbStubStateMachine::CtrlCInterrupt(s) => s.borrow_conn().socket() as _,
        GdbStubStateMachine::Disconnected(s) => s.borrow_conn().socket() as _,
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_shutdown(vmm: *mut Vmm) {
    (*vmm).shutdown.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn vmm_shutting_down(vmm: *mut Vmm) -> bool {
    (*vmm).shutdown.load(Ordering::Relaxed)
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
    cpu: CpuManager<self::hv::Default, self::screen::Default>, // Drop first.
    screen: self::screen::Default,
    gdb: Option<
        GdbStubStateMachine<
            'static,
            CpuManager<self::hv::Default, self::screen::Default>,
            DebugClient,
        >,
    >,
    shutdown: Arc<AtomicBool>,
}

impl Drop for Vmm {
    fn drop(&mut self) {
        // Set shutdown flag before dropping the other fields so their background thread can stop
        // before they try to join with it.
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Contains objects required to render the screen.
#[repr(C)]
pub struct VmmScreen {
    #[cfg(not(target_os = "macos"))]
    pub vk_instance: usize,
    #[cfg(not(target_os = "macos"))]
    pub vk_device: usize,
    #[cfg(not(target_os = "macos"))]
    pub vk_surface: usize,
    #[cfg(target_os = "macos")]
    pub view: usize,
}

/// Encapsulates a function to handle VMM events.
#[derive(Clone, Copy)]
struct VmmEventHandler {
    fp: unsafe extern "C" fn(*const VmmEvent, *mut c_void),
    cx: *mut c_void,
}

impl VmmEventHandler {
    unsafe fn invoke(self, e: VmmEvent) {
        (self.fp)(&e, self.cx)
    }
}

unsafe impl Send for VmmEventHandler {}
unsafe impl Sync for VmmEventHandler {}

/// Contains VMM event information.
#[repr(C)]
#[allow(dead_code)] // TODO: Figure out why Rust think fields in each enum are not used.
pub enum VmmEvent {
    Error {
        reason: *const RustError,
    },
    Exiting {
        success: bool,
    },
    Log {
        ty: VmmLog,
        data: *const c_char,
        len: usize,
    },
    Breakpoint {
        stop: *mut KernelStop,
    },
}

/// Log category.
///
/// The reason we need this because cbindgen is not good at exporting dependency types so we can't
/// use [`ConsoleType`] directly. See https://github.com/mozilla/cbindgen/issues/667 for an example
/// of the problem.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum VmmLog {
    Info,
    Warn,
    Error,
}

impl From<ConsoleType> for VmmLog {
    fn from(value: ConsoleType) -> Self {
        match value {
            ConsoleType::Info => Self::Info,
            ConsoleType::Warn => Self::Warn,
            ConsoleType::Error => Self::Error,
        }
    }
}

/// Reason for [`VmmEvent::Breakpoint`].
#[allow(dead_code)]
pub struct KernelStop(MultiThreadStopReason<u64>);

/// Result of [`vmm_dispatch_debug()`].
#[allow(dead_code)]
#[repr(C)]
pub enum DebugResult {
    Ok,
    Disconnected,
    Error { reason: *mut RustError },
}

/// Represents an error when [`vmm_new()`] fails.
#[derive(Debug, Error)]
enum VmmError {
    #[cfg(not(target_os = "macos"))]
    #[error("couldn't create Vulkan device")]
    CreateVulkanDeviceFailed(#[source] ash::vk::Result),

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
    CreateVmFailed(NonZero<applevisor_sys::hv_return_t>),

    #[cfg(target_os = "macos")]
    #[error("couldn't read ID_AA64MMFR0_EL1 ({0:#x})")]
    ReadMmfr0Failed(NonZero<applevisor_sys::hv_return_t>),

    #[cfg(target_os = "macos")]
    #[error("couldn't read ID_AA64MMFR1_EL1 ({0:#x})")]
    ReadMmfr1Failed(NonZero<applevisor_sys::hv_return_t>),

    #[cfg(target_os = "macos")]
    #[error("couldn't read ID_AA64MMFR2_EL1 ({0:#x})")]
    ReadMmfr2Failed(NonZero<applevisor_sys::hv_return_t>),

    #[cfg(target_os = "macos")]
    #[error("couldn't map memory to the VM ({0:#x})")]
    MapRamFailed(NonZero<applevisor_sys::hv_return_t>),

    #[cfg(target_os = "macos")]
    #[error("couldn't get default MTLDevice")]
    GetMetalDeviceFailed,
}

/// Represents an error when [`main_cpu()`] fails to reach event loop.
#[derive(Debug, Error)]
enum MainCpuError {
    #[error("couldn't get vCPU states")]
    GetCpuStatesFailed(#[source] Box<dyn Error + Send>),

    #[cfg(target_arch = "aarch64")]
    #[error("vCPU does not support {0:#x} page size")]
    PageSizeNotSupported(NonZero<usize>),

    #[cfg(target_arch = "aarch64")]
    #[error("physical address supported by vCPU too small")]
    PhysicalAddressTooSmall,

    #[error("couldn't commit vCPU states")]
    CommitCpuStatesFailed(#[source] Box<dyn Error + Send>),
}
