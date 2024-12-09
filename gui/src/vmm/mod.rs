// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::CpuManager;
use self::hw::{setup_devices, Device};
use self::kernel::{
    Kernel, PT_DYNAMIC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_STACK, PT_LOAD, PT_NOTE, PT_PHDR,
};
use self::ram::RamBuilder;
use crate::debug::DebugClient;
use crate::hv::{Hypervisor, Ram};
use crate::profile::Profile;
use cpu::GdbError;
use gdbstub::common::Signal;
use gdbstub::stub::state_machine::GdbStubStateMachine;
use gdbstub::stub::{GdbStubError, MultiThreadStopReason};
use kernel::{KernelError, ProgramHeaderError};
use obconf::{BootEnv, ConsoleType, Vm};
use std::cmp::max;
use std::error::Error;
use std::ffi::CStr;
use std::io::Read;
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use winit::event_loop::EventLoopProxy;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod debug;
mod hw;
mod kernel;
mod ram;

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
pub struct Vmm<'a, 'b> {
    cpu: CpuManager<'a, 'b, crate::hv::Default>, // Drop first.
    gdb: Option<GdbStubStateMachine<'static, CpuManager<'a, 'b, crate::hv::Default>, DebugClient>>,
    shutdown: Arc<AtomicBool>,
}

impl<'a, 'b> Vmm<'a, 'b> {
    pub fn new(args: VmmArgs<'b>, scope: &'a std::thread::Scope<'a, 'b>) -> Result<Self, VmmError> {
        let path = &args.kernel;
        let debugger = args.debugger;

        // Open kernel image.
        let mut kernel_img =
            Kernel::open(path).map_err(|e| VmmError::OpenKernel(e, path.to_path_buf()))?;

        // Get program header enumerator.
        let hdrs = kernel_img
            .program_headers()
            .map_err(|e| VmmError::EnumerateProgramHeaders(e, path.to_path_buf()))?;

        // Parse program headers.
        let mut segments = Vec::new();
        let mut dynamic = None;
        let mut note = None;

        for (index, item) in hdrs.enumerate() {
            // Check if success.
            let hdr =
                item.map_err(|e| VmmError::ReadProgramHeader(e, index, path.to_path_buf()))?;

            // Process the header.
            match hdr.p_type {
                PT_LOAD => {
                    if hdr.p_filesz > u64::try_from(hdr.p_memsz).unwrap() {
                        return Err(VmmError::InvalidFilesz(index));
                    }

                    segments.push(hdr);
                }
                PT_DYNAMIC => {
                    if dynamic.is_some() {
                        return Err(VmmError::MultipleDynamic);
                    }

                    dynamic = Some(hdr);
                }
                PT_NOTE => {
                    if note.is_some() {
                        return Err(VmmError::MultipleNote);
                    }

                    note = Some(hdr);
                }
                PT_PHDR | PT_GNU_EH_FRAME | PT_GNU_STACK | PT_GNU_RELRO => {}
                v => return Err(VmmError::UnknownProgramHeaderType(v, index)),
            }
        }

        segments.sort_unstable_by_key(|i| i.p_vaddr);

        // Make sure the first PT_LOAD includes the ELF header.
        let hdr = segments.first().ok_or(VmmError::NoLoadSegment)?;

        if hdr.p_offset != 0 {
            return Err(VmmError::ElfHeaderNotInFirstLoadSegment);
        }

        // Check if PT_DYNAMIC exists.
        let dynamic = dynamic.ok_or(VmmError::NoDynamicSegment)?;

        // Check if PT_NOTE exists.
        let note = note.ok_or(VmmError::NoNoteSegment)?;

        // Seek to PT_NOTE.
        let mut data: std::io::Take<&mut std::fs::File> = kernel_img
            .segment_data(&note)
            .map_err(|e| VmmError::SeekToNote(e, path.to_path_buf()))?;

        // Parse PT_NOTE.
        let mut vm_page_size = None;

        for i in 0u32.. {
            // Check remaining data.
            if data.limit() == 0 {
                break;
            }

            // Read note header.
            let mut buf = [0u8; 4 * 3];

            data.read_exact(&mut buf)
                .map_err(|e| VmmError::ReadKernelNote(e, i))?;

            // Parse note header.
            let nlen: usize = u32::from_ne_bytes(buf[..4].try_into().unwrap())
                .try_into()
                .unwrap();
            let dlen: usize = u32::from_ne_bytes(buf[4..8].try_into().unwrap())
                .try_into()
                .unwrap();
            let ty = u32::from_ne_bytes(buf[8..].try_into().unwrap());

            if nlen > 0xff {
                return Err(VmmError::NoteNameTooLarge(i));
            }

            if dlen > 0xff {
                return Err(VmmError::InvalidNoteDescription(i));
            }

            // Read note name + description.
            let nalign = nlen.next_multiple_of(4);
            let mut buf = vec![0u8; nalign + dlen];

            data.read_exact(&mut buf)
                .map_err(|e| VmmError::ReadKernelNoteData(e, i))?;

            // Check name.
            let name = match CStr::from_bytes_until_nul(&buf) {
                Ok(v) if v.to_bytes_with_nul().len() == nlen => v,
                _ => return Err(VmmError::InvalidNoteName(i)),
            };

            if name.to_bytes() != b"obkrnl" {
                continue;
            }

            // Parse description.
            match ty {
                0 => {
                    if vm_page_size.is_some() {
                        return Err(VmmError::DuplicateKernelNote(i));
                    }
                    vm_page_size = buf[nalign..]
                        .try_into()
                        .map(usize::from_ne_bytes)
                        .ok()
                        .and_then(NonZero::new)
                        .filter(|v| v.is_power_of_two());

                    if vm_page_size.is_none() {
                        return Err(VmmError::InvalidNoteDescription(i));
                    }
                }
                v => return Err(VmmError::UnknownKernelNoteType(v, i)),
            }
        }

        // Check if page size exists.
        let vm_page_size = vm_page_size.ok_or(VmmError::NoPageSizeInKernelNote)?;

        // Get page size on the host.
        let host_page_size = get_page_size().map_err(VmmError::GetHostPageSize)?;

        // Get kernel memory size.
        let mut len = 0;

        for hdr in &segments {
            if hdr.p_vaddr < len {
                return Err(VmmError::OverlappedLoadSegment(hdr.p_vaddr));
            }

            len = hdr
                .p_vaddr
                .checked_add(hdr.p_memsz)
                .ok_or(VmmError::InvalidPmemsz(hdr.p_vaddr))?;
        }

        // Round kernel memory size.
        let block_size = max(vm_page_size, host_page_size);
        let len = NonZero::new(len)
            .ok_or(VmmError::ZeroLengthLoadSegment)?
            .get()
            .checked_next_multiple_of(block_size.get())
            .ok_or(VmmError::TotalSizeTooLarge)?;

        // Setup RAM.
        let ram_size = NonZero::new(1024 * 1024 * 1024 * 8).unwrap();

        // Setup virtual devices.
        let devices = Arc::new(setup_devices(ram_size.get(), block_size));

        // Setup hypervisor.
        let mut hv = unsafe { crate::hv::new(8, ram_size, block_size, debugger.is_some()) }
            .map_err(VmmError::SetupHypervisor)?;

        // Map the kernel.
        let feats = hv.cpu_features().clone();
        let mut ram = RamBuilder::new(hv.ram_mut());

        let kern = ram
            .alloc_kernel(NonZero::new(len).unwrap())
            .map_err(VmmError::AllocateRamForKernel)?;

        for hdr in &segments {
            // Seek to segment data.
            let mut data = kernel_img
                .segment_data(hdr)
                .map_err(VmmError::SeekToOffset)?;

            // Read segment data.
            let mut seg = &mut kern[hdr.p_vaddr..(hdr.p_vaddr + hdr.p_memsz)];

            match std::io::copy(&mut data, &mut seg) {
                Ok(v) => {
                    if v != hdr.p_filesz {
                        return Err(VmmError::IncompleteKernel(path.to_path_buf()));
                    }
                }
                Err(e) => return Err(VmmError::ReadKernel(e, hdr.p_offset)),
            }
        }

        // Allocate stack.
        ram.alloc_stack(NonZero::new(1024 * 1024 * 2).unwrap())
            .map_err(VmmError::AllocateRamForStack)?;

        // Allocate arguments.
        let env = BootEnv::Vm(Vm {
            vmm: devices.vmm().addr(),
            console: devices.console().addr(),
            host_page_size,
        });

        ram.alloc_args(env, args.profile.kernel_config().clone())
            .map_err(VmmError::AllocateRamForArgs)?;

        // Build RAM.
        let map = ram
            .build(&feats, vm_page_size, &devices, dynamic)
            .map_err(VmmError::BuildRam)?;

        // Setup CPU manager.
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut cpu = CpuManager::new(Arc::new(hv), args.el, scope, devices, shutdown.clone());

        // Setup GDB stub.
        let gdb = debugger
            .map(|client| {
                gdbstub::stub::GdbStub::new(client)
                    .run_state_machine(&mut cpu)
                    .map_err(VmmError::SetupGdbStub)
            })
            .transpose()?;

        // Spawn main CPU.
        cpu.spawn(
            map.kern_vaddr + kernel_img.entry(),
            Some(map),
            gdb.is_some(),
        );

        // Create VMM.
        Ok(Self { cpu, gdb, shutdown })
    }

    fn dispatch_debug(
        &mut self,
        mut stop: Option<MultiThreadStopReason<u64>>,
    ) -> Result<DispatchDebugResult, DispatchDebugError> {
        loop {
            // Check current state.
            let r = match self.gdb.take().unwrap() {
                GdbStubStateMachine::Idle(s) => {
                    match self.cpu.dispatch_gdb_idle(s) {
                        Ok(Ok(v)) => Ok(v),
                        Ok(Err(v)) => {
                            // No pending data from the debugger.
                            self.gdb = Some(v.into());
                            return Ok(DispatchDebugResult::Ok);
                        }
                        Err(e) => Err(DispatchDebugError::DispatchIdle(e)),
                    }
                }
                GdbStubStateMachine::Running(s) => {
                    match self.cpu.dispatch_gdb_running(s, stop.take()) {
                        Ok(Ok(v)) => Ok(v),
                        Ok(Err(v)) => {
                            // No pending data from the debugger.
                            self.gdb = Some(v.into());
                            return Ok(DispatchDebugResult::Ok);
                        }
                        Err(e) => Err(DispatchDebugError::DispatchRunning(e)),
                    }
                }
                GdbStubStateMachine::CtrlCInterrupt(s) => {
                    self.cpu.lock();

                    s.interrupt_handled(
                        &mut self.cpu,
                        Some(MultiThreadStopReason::Signal(Signal::SIGINT)),
                    )
                    .map_err(DispatchDebugError::HandleInterrupt)
                }
                GdbStubStateMachine::Disconnected(_) => {
                    return Ok(DispatchDebugResult::Disconnected)
                }
            };

            match r {
                Ok(v) => self.gdb = Some(v),
                Err(e) => return Err(e),
            }
        }
    }
}

impl<'a, 'b> Drop for Vmm<'a, 'b> {
    fn drop(&mut self) {
        // Set shutdown flag before dropping the other fields so their background thread can stop
        // before they try to join with it.
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Encapsulates arguments for [`Vmm::new()`].
pub struct VmmArgs<'a> {
    pub profile: &'a Profile,
    pub kernel: PathBuf,
    pub debugger: Option<DebugClient>,
    pub el: EventLoopProxy<VmmEvent>,
}

/// Event from VMM.
#[derive(Debug)]
pub enum VmmEvent {
    Error {
        cpu: usize,
        reason: Box<dyn Error + Send>,
    },
    Exiting {
        success: bool,
    },
    Log(ConsoleType, String),
    Breakpoint(Option<MultiThreadStopReason<u64>>),
}

impl VmmEvent {
    fn error(cpu: usize, reason: impl Error + Send + 'static) -> Self {
        Self::Error {
            cpu,
            reason: Box::new(reason),
        }
    }
}

pub enum DispatchDebugResult {
    Ok,
    Disconnected,
}

#[derive(Debug, Error)]
enum DispatchDebugError {
    #[error("couldn't dispatch idle state")]
    DispatchIdle(#[source] debug::DispatchGdbIdleError),

    #[error("couldn't dispatch running state")]
    DispatchRunning(#[source] debug::DispatchGdbRunningError),

    #[error("couldn't handle CTRL+C interrupt")]
    HandleInterrupt(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),
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

#[derive(Debug, Error)]
pub enum VmmError {
    #[error("couldn't open kernel path {1}")]
    OpenKernel(#[source] KernelError, PathBuf),

    #[error("couldn't start enumerating program headers of {1}")]
    EnumerateProgramHeaders(#[source] std::io::Error, PathBuf),

    #[error("couldn't read program header #{1} on {2}")]
    ReadProgramHeader(#[source] ProgramHeaderError, usize, PathBuf),

    #[error("invalid p_filesz on on PT_LOAD {0}")]
    InvalidFilesz(usize),

    #[error("multiple PT_DYNAMIC is not supported")]
    MultipleDynamic,

    #[error("multiple PT_NOTE is not supported")]
    MultipleNote,

    #[error("unknown p_type {0} on program header {1}")]
    UnknownProgramHeaderType(u32, usize),

    #[error("the first PT_LOAD does not include ELF header")]
    ElfHeaderNotInFirstLoadSegment,

    #[error("no PT_LOAD on the kernel")]
    NoLoadSegment,

    #[error("no PT_DYNAMIC on the kernel")]
    NoDynamicSegment,

    #[error("no PT_NOTE on the kernel")]
    NoNoteSegment,

    #[error("couldn't seek to PT_NOTE on {1}")]
    SeekToNote(#[source] std::io::Error, PathBuf),

    #[error("couldn't read kernel note #{1}")]
    ReadKernelNote(#[source] std::io::Error, u32),

    #[error("name on kernel note #{0} is too large")]
    NoteNameTooLarge(u32),

    #[error("invalid description on kernel note #{0}")]
    InvalidNoteDescription(u32),

    #[error("couldn't read kernel note #{1} data")]
    ReadKernelNoteData(#[source] std::io::Error, u32),

    #[error("kernel note #{0} has invalid name")]
    InvalidNoteName(u32),

    #[error("kernel note #{0} is duplicated")]
    DuplicateKernelNote(u32),

    #[error("unknown type {0} on kernel note #{1}")]
    UnknownKernelNoteType(u32, u32),

    #[error("no page size in kernel note")]
    NoPageSizeInKernelNote,

    #[error("couldn't get host page size")]
    GetHostPageSize(#[source] std::io::Error),

    #[error("PT_LOAD at {0:#} is overlapped with the previous PT_LOAD")]
    OverlappedLoadSegment(usize),

    #[error("invalid p_memsz on PT_LOAD at {0:#}")]
    InvalidPmemsz(usize),

    #[error("the kernel has PT_LOAD with zero length")]
    ZeroLengthLoadSegment,

    #[error("total size of PT_LOAD is too large")]
    TotalSizeTooLarge,

    #[error("couldn't setup a hypervisor")]
    SetupHypervisor(#[source] crate::hv::HypervisorError),

    #[error("couldn't allocate RAM for the kernel")]
    AllocateRamForKernel(#[source] crate::hv::RamError),

    #[error("couldn't seek to offset")]
    SeekToOffset(#[source] std::io::Error),

    #[error("{0} is incomplete")]
    IncompleteKernel(PathBuf),

    #[error("couldn't read kernel at offset {1}")]
    ReadKernel(#[source] std::io::Error, u64),

    #[error("couldn't allocate RAM for stack")]
    AllocateRamForStack(#[source] crate::hv::RamError),

    #[error("couldn't allocate RAM for arguments")]
    AllocateRamForArgs(#[source] crate::hv::RamError),

    #[error("couldn't build RAM")]
    BuildRam(#[source] ram::RamBuilderError),

    #[error("couldn't setup a GDB stub")]
    SetupGdbStub(
        #[source] GdbStubError<GdbError, <DebugClient as gdbstub::conn::Connection>::Error>,
    ),
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
