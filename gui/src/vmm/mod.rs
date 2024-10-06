// SPDX-License-Identifier: MIT OR Apache-2.0
use self::hv::{Cpu, CpuExit, CpuFeats, CpuIo, Hypervisor};
use self::hw::{setup_devices, Device, DeviceContext, DeviceTree};
use self::kernel::{
    Kernel, PT_DYNAMIC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_STACK, PT_LOAD, PT_NOTE, PT_PHDR,
};
use self::ram::{Ram, RamMap};
use self::screen::Screen;
use crate::error::RustError;
use crate::profile::Profile;
use gdbstub::stub::state_machine::state::Running;
use gdbstub::stub::state_machine::{GdbStubStateMachine, GdbStubStateMachineInner};
use gdbstub::stub::{GdbStub, MultiThreadStopReason};
use obconf::{BootEnv, ConsoleType, Vm};
use std::cmp::max;
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::{c_char, c_void, CStr};
use std::io::Read;
use std::net::TcpListener;
use std::num::NonZero;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod debug;
mod hv;
mod hw;
mod kernel;
mod ram;
mod screen;

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_run(
    kernel: *const c_char,
    screen: *const VmmScreen,
    profile: *const Profile,
    debug: *const c_char,
    event: unsafe extern "C" fn(*const VmmEvent, *mut c_void),
    cx: *mut c_void,
    err: *mut *mut RustError,
) -> *mut Vmm {
    // Setup debug server.
    let debug = match setup_debug_server(debug) {
        Ok(v) => v,
        Err(e) => {
            *err = e.into_c();
            return null_mut();
        }
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
    let ram = match Ram::new(block_size) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't create a RAM", e).into_c();
            return null_mut();
        }
    };

    // Setup hypervisor.
    let mut hv = match self::hv::new(8, ram) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't setup a hypervisor", e).into_c();
            return null_mut();
        }
    };

    // Load CPU features.
    let feats = match hv.cpu_features() {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't get available vCPU features", e).into_c();
            return null_mut();
        }
    };

    // Map the kernel.
    let mut ram = hv.ram_mut().builder();
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
    let event = VmmEventHandler { fp: event, cx };
    let devices = Arc::new(setup_devices(Ram::SIZE, block_size, event));
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

    // Setup arguments for main CPU.
    let shutdown = Arc::new(AtomicBool::new(false));
    let args = CpuArgs {
        hv,
        screen: screen.buffer().clone(),
        feats,
        devices,
        event,
        shutdown: shutdown.clone(),
    };

    // Spawn a thread to drive main CPU.
    let e_entry = file.entry();
    let main = move || main_cpu(&args, e_entry, map, debug);
    let main = match std::thread::Builder::new().spawn(main) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't spawn main CPU", e).into_c();
            return null_mut();
        }
    };

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
        Err(e) => RustError::wrap(e).into_c(),
    }
}

/// # Safety
/// `addr` cannot be null and must point to a null-terminated string.
unsafe fn setup_debug_server(addr: *const c_char) -> Result<Option<TcpListener>, RustError> {
    // Get listen address.
    let addr = match addr.is_null() {
        true => return Ok(None),
        false => CStr::from_ptr(addr)
            .to_str()
            .map_err(|_| RustError::new("address to listen for a debugger is not UTF-8"))?,
    };

    // Setup server.
    let sock = TcpListener::bind(addr)
        .map_err(|e| RustError::with_source("couldn't listen for a debugger", e))?;

    sock.set_nonblocking(true)
        .map(|_| Some(sock))
        .map_err(|e| RustError::with_source("couldn't enable non-blocking on a debug server", e))
}

fn main_cpu<H: Hypervisor>(
    args: &CpuArgs<H>,
    entry: usize,
    map: RamMap,
    debug: Option<TcpListener>,
) {
    // Create vCPU.
    let mut cpu = match args.hv.create_cpu(0) {
        Ok(v) => v,
        Err(e) => {
            let e = RustError::with_source("couldn't create main CPU", e);
            unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
            return;
        }
    };

    if let Err(e) = self::arch::setup_main_cpu(&mut cpu, entry, map, &args.feats) {
        let e = RustError::with_source("couldn't setup main CPU", e);
        unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
        return;
    }

    // Check if debug.
    let debug = match debug {
        Some(v) => v,
        None => {
            run_cpu(cpu, args, None);
            return;
        }
    };

    // Get server address.
    let addr = match debug.local_addr() {
        Ok(v) => v.to_string(),
        Err(e) => {
            let e = RustError::with_source("couldn't get debug server address", e);
            unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
            return;
        }
    };

    // Tell the user to connect a debugger.
    let len = addr.len();
    let addr = addr.as_ptr().cast();

    unsafe { args.event.invoke(VmmEvent::WaitingDebugger { addr, len }) };

    // Wait for a debugger.
    let client = loop {
        use std::io::ErrorKind;

        if args.shutdown.load(Ordering::Relaxed) {
            return;
        }

        // Try accept a connection.
        let e = match debug.accept() {
            Ok(v) => break self::debug::Client::new(v.0),
            Err(e) => e,
        };

        match e.kind() {
            ErrorKind::WouldBlock => sleep(Duration::from_millis(500)),
            ErrorKind::Interrupted => {}
            _ => {
                let e = RustError::with_source("couldn't accept a debugger connection", e);
                unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                return;
            }
        }
    };

    // Setup GDB stub.
    let mut target = self::debug::Target::new();
    let gdb = match GdbStub::new(client).run_state_machine(&mut target) {
        Ok(v) => v,
        Err(e) => {
            let e = RustError::with_source("couldn't setup a GDB stub", e);
            unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
            return;
        }
    };

    // Run GDB until the client is waiting for the target to report a stop reason.
    let gdb = match run_gdb(args, &mut target, gdb) {
        Some(v) => v,
        None => return,
    };

    run_cpu(cpu, args, Some(gdb));
}

fn run_cpu<H: Hypervisor>(
    mut cpu: H::Cpu<'_>,
    args: &CpuArgs<H>,
    gdb: Option<GdbStubStateMachineInner<Running, self::debug::Target, self::debug::Client>>,
) {
    // Build device contexts for this CPU.
    let mut devices = args
        .devices
        .map()
        .map(|(addr, dev)| {
            let end = dev.len().checked_add(addr).unwrap();

            (addr, (dev.create_context(&args.hv), end))
        })
        .collect::<BTreeMap<usize, (Box<dyn DeviceContext>, NonZero<usize>)>>();

    // Dispatch CPU events until shutdown.
    while !args.shutdown.load(Ordering::Relaxed) {
        // Run the vCPU.
        let exit = match cpu.run() {
            Ok(v) => v,
            Err(e) => {
                let e = RustError::with_source("couldn't run CPU", e);
                unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                break;
            }
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

fn run_gdb<'a, H: Hypervisor>(
    args: &CpuArgs<H>,
    target: &mut self::debug::Target,
    mut state: GdbStubStateMachine<'a, self::debug::Target, self::debug::Client>,
) -> Option<GdbStubStateMachineInner<'a, Running, self::debug::Target, self::debug::Client>> {
    loop {
        // Check current state.
        let mut s = match state {
            GdbStubStateMachine::Idle(s) => s,
            GdbStubStateMachine::Running(s) => return Some(s),
            GdbStubStateMachine::CtrlCInterrupt(s) => {
                state = match s.interrupt_handled(target, None::<MultiThreadStopReason<u64>>) {
                    Ok(v) => v,
                    Err(e) => {
                        let e = RustError::with_source("couldn't handle CTRL+C from a debugger", e);
                        unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                        return None;
                    }
                };

                continue;
            }
            GdbStubStateMachine::Disconnected(_) => {
                unsafe { args.event.invoke(VmmEvent::DebuggerDisconnected) };
                return None;
            }
        };

        // Read data from the client.
        let b = match s.borrow_conn().read() {
            Ok(v) => v,
            Err(e) => {
                let e = RustError::with_source("couldn't read data from a debugger", e);
                unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                return None;
            }
        };

        state = match s.incoming_data(target, b) {
            Ok(v) => v,
            Err(e) => {
                let e = RustError::with_source("couldn't process data from a debugger", e);
                unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                return None;
            }
        };
    }
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
    WaitingDebugger {
        addr: *const c_char,
        len: usize,
    },
    DebuggerDisconnected,
    Exiting {
        success: bool,
    },
    Log {
        ty: VmmLog,
        data: *const c_char,
        len: usize,
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

/// Encapsulates arguments for a function to run a CPU.
struct CpuArgs<H: Hypervisor> {
    hv: H,
    screen: Arc<<self::screen::Default as Screen>::Buffer>,
    feats: CpuFeats,
    devices: Arc<DeviceTree<H>>,
    event: VmmEventHandler,
    shutdown: Arc<AtomicBool>,
}

/// Represents an error when [`vmm_new()`] fails.
#[derive(Debug, Error)]
enum VmmError {
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

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("your OS does not support KVM_CAP_ONE_REG")]
    NoKvmOneReg,

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("your OS does not support KVM_CAP_ARM_VM_IPA_SIZE")]
    NoVmIpaSize,

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("physical address supported by your CPU too small")]
    PhysicalAddressTooSmall,

    #[cfg(target_os = "linux")]
    #[error("couldn't create a VM")]
    CreateVmFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't map the RAM to the VM")]
    MapRamFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't create vCPU #{0}")]
    CreateCpuFailed(usize, #[source] std::io::Error),

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("couldn't initialize vCPU #{0}")]
    InitCpuFailed(usize, #[source] std::io::Error),

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("couldn't read ID_AA64MMFR0_EL1")]
    ReadMmfr0Failed(#[source] std::io::Error),

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("couldn't read ID_AA64MMFR1_EL1")]
    ReadMmfr1Failed(#[source] std::io::Error),

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("couldn't read ID_AA64MMFR2_EL1")]
    ReadMmfr2Failed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't get the size of vCPU mmap")]
    GetMmapSizeFailed(#[source] std::io::Error),

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[error("couldn't get preferred CPU target")]
    GetPreferredTargetFailed(#[source] std::io::Error),

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
