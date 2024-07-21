use self::ram::Ram;
use crate::error::RustError;
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

#[no_mangle]
pub unsafe extern "C" fn vmm_new(err: *mut *mut RustError) -> *mut Vmm {
    // Setup RAM.
    let ram = match Ram::new(0) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            *err = RustError::wrap(HypervisorError::CreateRamFailed(e));
            return null_mut();
        }
    };

    // Setup hypervisor.
    let platform = match setup_platform(8, ram.clone()) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::wrap(e);
            return null_mut();
        }
    };

    // Create VMM.
    let vmm = Vmm {
        platform,
        ram,
        created_cpu: AtomicUsize::new(0),
    };

    Box::into_raw(vmm.into())
}

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[cfg(target_os = "linux")]
fn setup_platform(cpu: usize, ram: Arc<Ram>) -> Result<self::linux::Kvm, HypervisorError> {
    self::linux::Kvm::new(cpu, ram)
}

#[cfg(target_os = "windows")]
fn setup_platform(cpu: usize, ram: Arc<Ram>) -> Result<self::windows::Whp, HypervisorError> {
    self::windows::Whp::new(cpu, ram)
}

#[cfg(target_os = "macos")]
fn setup_platform(cpu: usize, ram: Arc<Ram>) -> Result<self::macos::Hf, HypervisorError> {
    self::macos::Hf::new(cpu, ram)
}

/// Manage a virtual machine that run the kernel.
pub struct Vmm {
    platform: P,
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
enum HypervisorError {
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
