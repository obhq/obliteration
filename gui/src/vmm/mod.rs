// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::CpuManager;
use self::hv::Hypervisor;
use self::hw::{setup_devices, Device};
use self::kernel::{
    Kernel, PT_DYNAMIC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_STACK, PT_LOAD, PT_NOTE, PT_PHDR,
};
use self::ram::{Ram, RamBuilder};
use crate::debug::DebugClient;
use crate::error::RustError;
use crate::profile::Profile;
use crate::screen::Screen;
use cpu::GdbError;
use gdbstub::stub::state_machine::GdbStubStateMachine;
use gdbstub::stub::{GdbStubError, MultiThreadStopReason};
use kernel::{KernelError, ProgramHeaderError};
use obconf::{BootEnv, ConsoleType, Vm};
use std::cmp::max;
use std::error::Error;
use std::ffi::{c_char, c_void, CStr};
use std::io::Read;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod debug;
#[cfg(feature = "qt_ffi")]
mod ffi;
mod hv;
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
pub struct Vmm {
    cpu: CpuManager<self::hv::Default, crate::screen::Default>, // Drop first.
    screen: crate::screen::Default,
    gdb: Option<
        GdbStubStateMachine<
            'static,
            CpuManager<self::hv::Default, crate::screen::Default>,
            DebugClient,
        >,
    >,
    shutdown: Arc<AtomicBool>,
}

impl Vmm {
    pub fn new(
        kernel_path: impl AsRef<Path>,
        screen: &VmmScreen,
        profile: &Profile,
        debugger: Option<DebugClient>,
        event: unsafe extern "C" fn(*const VmmEvent, *mut c_void),
        cx: *mut c_void,
    ) -> Result<Self, StartVmmError> {
        let path = kernel_path.as_ref();

        // Open kernel image.
        let mut kernel_img =
            Kernel::open(path).map_err(|e| StartVmmError::OpenKernel(e, path.to_path_buf()))?;

        // Get program header enumerator.
        let hdrs = kernel_img
            .program_headers()
            .map_err(|e| StartVmmError::EnumerateProgramHeaders(e, path.to_path_buf()))?;

        // Parse program headers.
        let mut segments = Vec::new();
        let mut dynamic = None;
        let mut note = None;

        for (index, item) in hdrs.enumerate() {
            // Check if success.
            let hdr =
                item.map_err(|e| StartVmmError::ReadProgramHeader(e, index, path.to_path_buf()))?;

            // Process the header.
            match hdr.p_type {
                PT_LOAD => {
                    if hdr.p_filesz > u64::try_from(hdr.p_memsz).unwrap() {
                        return Err(StartVmmError::InvalidFilesz(index));
                    }

                    segments.push(hdr);
                }
                PT_DYNAMIC => {
                    if dynamic.is_some() {
                        return Err(StartVmmError::MultipleDynamic);
                    }

                    dynamic = Some(hdr);
                }
                PT_NOTE => {
                    if note.is_some() {
                        return Err(StartVmmError::MultipleNote);
                    }

                    note = Some(hdr);
                }
                PT_PHDR | PT_GNU_EH_FRAME | PT_GNU_STACK | PT_GNU_RELRO => {}
                v => return Err(StartVmmError::UnknownProgramHeaderType(v, index)),
            }
        }

        segments.sort_unstable_by_key(|i| i.p_vaddr);

        // Make sure the first PT_LOAD includes the ELF header.
        let hdr = segments.first().ok_or(StartVmmError::NoLoadSegment)?;

        if hdr.p_offset != 0 {
            return Err(StartVmmError::ElfHeaderNotInFirstLoadSegment);
        }

        // Check if PT_DYNAMIC exists.
        let dynamic = dynamic.ok_or(StartVmmError::NoDynamicSegment)?;

        // Check if PT_NOTE exists.
        let note = note.ok_or(StartVmmError::NoNoteSegment)?;

        // Seek to PT_NOTE.
        let mut data: std::io::Take<&mut std::fs::File> = kernel_img
            .segment_data(&note)
            .map_err(|e| StartVmmError::SeekToNote(e, path.to_path_buf()))?;

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
                .map_err(|e| StartVmmError::ReadKernelNote(e, i))?;

            // Parse note header.
            let nlen: usize = u32::from_ne_bytes(buf[..4].try_into().unwrap())
                .try_into()
                .unwrap();
            let dlen: usize = u32::from_ne_bytes(buf[4..8].try_into().unwrap())
                .try_into()
                .unwrap();
            let ty = u32::from_ne_bytes(buf[8..].try_into().unwrap());

            if nlen > 0xff {
                return Err(StartVmmError::NoteNameTooLarge(i));
            }

            if dlen > 0xff {
                return Err(StartVmmError::InvalidNoteDescription(i));
            }

            // Read note name + description.
            let nalign = nlen.next_multiple_of(4);
            let mut buf = vec![0u8; nalign + dlen];

            data.read_exact(&mut buf)
                .map_err(|e| StartVmmError::ReadKernelNoteData(e, i))?;

            // Check name.
            let name = match CStr::from_bytes_until_nul(&buf) {
                Ok(v) if v.to_bytes_with_nul().len() == nlen => v,
                _ => return Err(StartVmmError::InvalidNoteName(i)),
            };

            if name.to_bytes() != b"obkrnl" {
                continue;
            }

            // Parse description.
            match ty {
                0 => {
                    if vm_page_size.is_some() {
                        return Err(StartVmmError::DuplicateKernelNote(i));
                    }

                    vm_page_size = buf[nalign..]
                        .try_into()
                        .map(usize::from_ne_bytes)
                        .ok()
                        .and_then(NonZero::new)
                        .filter(|v| v.is_power_of_two());

                    if vm_page_size.is_none() {
                        return Err(StartVmmError::InvalidNoteDescription(i));
                    }
                }
                v => return Err(StartVmmError::UnknownKernelNoteType(v, i)),
            }
        }

        // Check if page size exists.
        let vm_page_size = vm_page_size.ok_or(StartVmmError::NoPageSizeInKernelNote)?;

        // Get page size on the host.
        let host_page_size = get_page_size().map_err(StartVmmError::GetHostPageSize)?;

        // Get kernel memory size.
        let mut len = 0;

        for hdr in &segments {
            if hdr.p_vaddr < len {
                return Err(StartVmmError::OverlappedLoadSegment(hdr.p_vaddr));
            }

            len = hdr
                .p_vaddr
                .checked_add(hdr.p_memsz)
                .ok_or(StartVmmError::InvalidPmemsz(hdr.p_vaddr))?;
        }

        // Round kernel memory size.
        let block_size = max(vm_page_size, host_page_size);
        let len = NonZero::new(len)
            .ok_or(StartVmmError::ZeroLengthLoadSegment)?
            .get()
            .checked_next_multiple_of(block_size.get())
            .ok_or(StartVmmError::TotalSizeTooLarge)?;

        // Setup RAM.
        let ram = unsafe { Ram::new(NonZero::new(1024 * 1024 * 1024 * 8).unwrap(), block_size) }
            .map_err(StartVmmError::CreateRam)?;

        // Setup virtual devices.
        let event = VmmEventHandler { event, cx };
        let devices = Arc::new(setup_devices(ram.len().get(), block_size, event));

        // Setup hypervisor.
        let mut hv =
            self::hv::new(8, ram, debugger.is_some()).map_err(StartVmmError::SetupHypervisor)?;

        // Map the kernel.
        let feats = hv.cpu_features().clone();
        let mut ram = RamBuilder::new(hv.ram_mut());

        let kern = ram
            .alloc_kernel(NonZero::new(len).unwrap())
            .map_err(StartVmmError::AllocateRamForKernel)?;

        for hdr in &segments {
            // Seek to segment data.
            let mut data = kernel_img
                .segment_data(hdr)
                .map_err(StartVmmError::SeekToOffset)?;

            // Read segment data.
            let mut seg = &mut kern[hdr.p_vaddr..(hdr.p_vaddr + hdr.p_memsz)];

            match std::io::copy(&mut data, &mut seg) {
                Ok(v) => {
                    if v != hdr.p_filesz {
                        return Err(StartVmmError::IncompleteKernel(path.to_path_buf()));
                    }
                }
                Err(e) => return Err(StartVmmError::ReadKernel(e, hdr.p_offset)),
            }
        }

        // Allocate stack.
        ram.alloc_stack(NonZero::new(1024 * 1024 * 2).unwrap())
            .map_err(StartVmmError::AllocateRamForStack)?;

        // Allocate arguments.
        let env = BootEnv::Vm(Vm {
            vmm: devices.vmm().addr(),
            console: devices.console().addr(),
            host_page_size,
        });

        ram.alloc_args(env, profile.kernel_config().clone())
            .map_err(StartVmmError::AllocateRamForArgs)?;

        // Build RAM.
        let map = ram
            .build(&feats, vm_page_size, &devices, dynamic)
            .map_err(StartVmmError::BuildRam)?;

        // Setup screen.
        let screen =
            crate::screen::Default::from_screen(screen).map_err(StartVmmError::SetupScreen)?;

        // Setup CPU manager.
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut cpu_manager = CpuManager::new(
            Arc::new(hv),
            screen.buffer().clone(),
            devices,
            event,
            shutdown.clone(),
        );

        // Setup GDB stub.
        let gdb = debugger
            .map(|client| {
                gdbstub::stub::GdbStub::new(client)
                    .run_state_machine(&mut cpu_manager)
                    .map_err(StartVmmError::SetupGdbStub)
            })
            .transpose()?;

        // Spawn main CPU.
        cpu_manager.spawn(
            map.kern_vaddr + kernel_img.entry(),
            Some(map),
            gdb.is_some(),
        );

        // Create VMM.
        let vmm = Vmm {
            cpu: cpu_manager,
            screen,
            gdb,
            shutdown,
        };

        Ok(vmm)
    }
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
    event: unsafe extern "C" fn(*const VmmEvent, *mut c_void),
    cx: *mut c_void,
}

impl VmmEventHandler {
    unsafe fn invoke(self, e: VmmEvent) {
        (self.event)(&e, self.cx);
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

#[derive(Debug, Error)]
pub enum StartVmmError {
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

    #[error("couldn't create a RAM")]
    CreateRam(#[source] std::io::Error),

    #[error("couldn't setup a hypervisor")]
    SetupHypervisor(#[source] hv::DefaultError),

    #[error("couldn't allocate RAM for the kernel")]
    AllocateRamForKernel(#[source] ram::RamError),

    #[error("couldn't seek to offset")]
    SeekToOffset(#[source] std::io::Error),

    #[error("{0} is incomplete")]
    IncompleteKernel(PathBuf),

    #[error("couldn't read kernel at offset {1}")]
    ReadKernel(#[source] std::io::Error, u64),

    #[error("couldn't allocate RAM for stack")]
    AllocateRamForStack(#[source] ram::RamError),

    #[error("couldn't allocate RAM for arguments")]
    AllocateRamForArgs(#[source] ram::RamError),

    #[error("couldn't build RAM")]
    BuildRam(#[source] ram::RamBuilderError),

    #[error("couldn't setup a screen")]
    SetupScreen(#[source] crate::screen::ScreenError),

    #[error("couldn't setup a GDB stub")]
    SetupGdbStub(
        #[source] GdbStubError<GdbError, <DebugClient as gdbstub::conn::Connection>::Error>,
    ),
}

/// Represents an error when [`vmm_new()`] fails.
#[derive(Debug, Error)]
enum VmmError {
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
