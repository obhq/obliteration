// SPDX-License-Identifier: MIT OR Apache-2.0
use self::channel::{create_channel, VmmStream};
use self::cpu::CpuManager;
use self::hw::{setup_devices, Device};
use self::kernel::{
    Kernel, NoteError, PT_DYNAMIC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_STACK, PT_LOAD, PT_NOTE,
    PT_PHDR,
};
use self::ram::RamBuilder;
use crate::gdb::DebugClient;
use crate::hv::{Hypervisor, Ram};
use crate::profile::Profile;
use gdbstub::stub::MultiThreadStopReason;
use kernel::{KernelError, ProgramHeaderError};
use obconf::{BootEnv, ConsoleType, Vm};
use std::cmp::max;
use std::error::Error;
use std::future::Future;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod channel;
mod cpu;
mod hw;
mod kernel;
mod ram;

/// Manage a virtual machine that run the kernel.
pub struct Vmm<H> {
    cpu: CpuManager<H>, // Drop first.
    shutdown: Arc<AtomicBool>,
    events: VmmStream,
}

impl Vmm<()> {
    pub fn new(
        profile: &Profile,
        kernel: &Path,
        debugger: Option<DebugClient>,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Vmm<impl Hypervisor>, VmmError> {
        // Get program header enumerator.
        let mut img = Kernel::open(kernel).map_err(|e| VmmError::OpenKernel(e))?;
        let hdrs = img
            .program_headers()
            .map_err(|e| VmmError::EnumerateProgramHeaders(e))?;

        // Parse program headers.
        let mut segments = Vec::new();
        let mut dynamic = None;
        let mut note = None;

        for (index, item) in hdrs.enumerate() {
            let hdr = item.map_err(|e| VmmError::ReadProgramHeader(index, e))?;

            match hdr.p_type {
                PT_LOAD => {
                    if hdr.p_filesz > hdr.p_memsz {
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
                PT_PHDR | PT_GNU_EH_FRAME | PT_GNU_STACK | PT_GNU_RELRO => (),
                v => return Err(VmmError::UnknownProgramHeaderType(v, index)),
            }
        }

        segments.sort_unstable_by_key(|i| i.p_vaddr);

        // Make sure the first PT_LOAD includes the ELF header.
        let hdr = segments.first().ok_or(VmmError::NoLoadSegment)?;

        if hdr.p_offset != 0 {
            return Err(VmmError::ElfHeaderNotInFirstLoadSegment);
        }

        // Check if PT_DYNAMIC and PT_NOTE exists.
        let dynamic = dynamic.ok_or(VmmError::NoDynamicSegment)?;
        let note = note.ok_or(VmmError::NoNoteSegment)?;

        // Parse PT_NOTE.
        let mut vm_page_size = None;

        if note.p_filesz > 1024 * 1024 {
            return Err(VmmError::NoteSegmentTooLarge);
        }

        for (i, note) in img.notes(&note).map_err(VmmError::SeekToNote)?.enumerate() {
            let note = note.map_err(move |e| VmmError::ReadKernelNote(i, e))?;

            if note.name.as_ref() != b"obkrnl" {
                continue;
            }

            match note.ty {
                0 => {
                    if vm_page_size.is_some() {
                        return Err(VmmError::DuplicateKernelNote(i));
                    }

                    vm_page_size = note
                        .desc
                        .as_ref()
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

        // Check if required notes exists.
        let vm_page_size = vm_page_size.ok_or(VmmError::NoPageSizeInKernelNote)?;

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
        let host_page_size = Self::get_page_size().map_err(VmmError::GetHostPageSize)?;
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
            let mut src = img
                .segment_data(hdr)
                .map_err(|e| VmmError::SeekToOffset(hdr.p_offset, e))?;
            let mut dst = &mut kern[hdr.p_vaddr..(hdr.p_vaddr + hdr.p_memsz)];

            match std::io::copy(&mut src, &mut dst) {
                Ok(v) => {
                    if v != u64::try_from(hdr.p_filesz).unwrap() {
                        return Err(VmmError::IncompleteKernel);
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

        ram.alloc_args(env, profile.kernel_config().clone())
            .map_err(VmmError::AllocateRamForArgs)?;

        // Build RAM.
        let map = ram
            .build(&feats, vm_page_size, &devices, dynamic)
            .map_err(VmmError::BuildRam)?;

        // Spawn main CPU.
        let (events, main) = create_channel();
        let mut cpu = CpuManager::new(Arc::new(hv), main, devices, shutdown.clone());

        cpu.spawn(map.kern_vaddr + img.entry(), Some(map), debugger.is_some());

        Ok(Vmm {
            cpu,
            shutdown: shutdown.clone(),
            events,
        })
    }
}

impl<H> Vmm<H> {
    pub fn recv(&mut self) -> impl Future<Output = Option<VmmEvent>> + '_ {
        self.events.recv()
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
}

impl<H> Drop for Vmm<H> {
    fn drop(&mut self) {
        // Set shutdown flag before dropping the other fields so their background thread can stop
        // before they try to join with it.
        self.shutdown.store(true, Ordering::Relaxed);
    }
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

/// Represents an error when [`Vmm::new()`] fails.
#[derive(Debug, Error)]
pub enum VmmError {
    #[error("couldn't open the kernel")]
    OpenKernel(#[source] KernelError),

    #[error("couldn't start enumerating program headers")]
    EnumerateProgramHeaders(#[source] std::io::Error),

    #[error("couldn't read program header #{0}")]
    ReadProgramHeader(usize, #[source] ProgramHeaderError),

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

    #[error("PT_NOTE is too large")]
    NoteSegmentTooLarge,

    #[error("couldn't seek to PT_NOTE")]
    SeekToNote(#[source] std::io::Error),

    #[error("couldn't read kernel note #{0}")]
    ReadKernelNote(usize, #[source] NoteError),

    #[error("invalid description on kernel note #{0}")]
    InvalidNoteDescription(usize),

    #[error("kernel note #{0} is duplicated")]
    DuplicateKernelNote(usize),

    #[error("unknown type {0} on kernel note #{1}")]
    UnknownKernelNoteType(u32, usize),

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

    #[error("couldn't seek to offset {0:#x}")]
    SeekToOffset(u64, #[source] std::io::Error),

    #[error("the kernel is incomplete")]
    IncompleteKernel,

    #[error("couldn't read kernel at offset {1}")]
    ReadKernel(#[source] std::io::Error, u64),

    #[error("couldn't allocate RAM for stack")]
    AllocateRamForStack(#[source] crate::hv::RamError),

    #[error("couldn't allocate RAM for arguments")]
    AllocateRamForArgs(#[source] crate::hv::RamError),

    #[error("couldn't build RAM")]
    BuildRam(#[source] ram::RamBuilderError),
}

/// Represents an error when [`main_cpu()`] fails to reach event loop.
#[derive(Debug, Error)]
enum MainCpuError {
    #[error("couldn't get vCPU states")]
    GetCpuStatesFailed(#[source] Box<dyn Error + Send + Sync>),

    #[cfg(target_arch = "aarch64")]
    #[error("vCPU does not support {0:#x} page size")]
    PageSizeNotSupported(NonZero<usize>),

    #[cfg(target_arch = "aarch64")]
    #[error("physical address supported by vCPU too small")]
    PhysicalAddressTooSmall,

    #[error("couldn't commit vCPU states")]
    CommitCpuStatesFailed(#[source] Box<dyn Error + Send + Sync>),
}
