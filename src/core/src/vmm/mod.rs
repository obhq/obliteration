use self::ram::Ram;
use crate::error::RustError;
use std::ffi::{c_char, CStr};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ptr::null_mut;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use thiserror::Error;

pub(self) use self::cpu::*;
pub(self) use self::platform::*;

mod cpu;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod platform;
mod ram;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_arch = "x86_64")]
const ELF_MACHINE: u16 = 62;
#[cfg(target_arch = "aarch64")]
const ELF_MACHINE: u16 = 183;
const KERNEL_PADDR: usize = 0; // TODO: Figure out where PS4 map the kernel.

#[no_mangle]
pub unsafe extern "C" fn vmm_new(err: *mut *mut RustError) -> *mut Vmm {
    // Setup RAM.
    let ram = match Ram::new(0) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Setup hypervisor.
    let hv = match setup_platform(8, ram.clone()) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Create VMM.
    let vmm = Vmm {
        hv,
        ram,
        created_cpu: AtomicUsize::new(0),
    };

    Box::into_raw(vmm.into())
}

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_run(vmm: *mut Vmm, kernel: *const c_char) -> *mut RustError {
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
    let e_entry = u64::from_ne_bytes(hdr[24..32].try_into().unwrap());
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
        Some(&(p_offset, _, p_vaddr, _)) => {
            if p_offset != 0 || p_vaddr != 0 {
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
    let stack_off = end; // TODO: Figure out how PS4 allocate the stack.
    let stack_len = 1024 * 1024 * 2; // TODO: Same here.
    let off = KERNEL_PADDR.checked_sub((*vmm).ram.vm_addr()).unwrap();
    let len = stack_off + stack_len;
    let mem = match (*vmm).ram.alloc(off, len) {
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

    // TODO: Set hypervisor to run the mapped kernel then start a thread to drive the vCPU.
    null_mut()
}

#[cfg(target_os = "linux")]
fn setup_platform(cpu: usize, ram: Arc<Ram>) -> Result<self::linux::Kvm, VmmError> {
    self::linux::Kvm::new(cpu, ram)
}

#[cfg(target_os = "windows")]
fn setup_platform(cpu: usize, ram: Arc<Ram>) -> Result<self::windows::Whp, VmmError> {
    self::windows::Whp::new(cpu, ram)
}

#[cfg(target_os = "macos")]
fn setup_platform(cpu: usize, ram: Arc<Ram>) -> Result<self::macos::Hf, VmmError> {
    self::macos::Hf::new(cpu, ram)
}

/// Manage a virtual machine that run the kernel.
pub struct Vmm {
    hv: P,
    ram: Arc<Ram>,
    created_cpu: AtomicUsize,
}

#[cfg(target_os = "linux")]
type P = self::linux::Kvm;

#[cfg(target_os = "windows")]
type P = self::windows::Whp;

#[cfg(target_os = "macos")]
type P = self::macos::Hf;

/// Object that has a physical address in the virtual machine.
trait MemoryAddr {
    /// Physical address in the virtual machine.
    fn vm_addr(&self) -> usize;

    /// Address in our process.
    fn host_addr(&self) -> *mut ();

    /// Total size of the object, in bytes.
    fn len(&self) -> usize;
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
}
