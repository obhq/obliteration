use self::platform::Platform;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;

pub use self::cpu::*;
pub use self::ram::*;

mod cpu;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod platform;
mod ram;
#[cfg(target_os = "windows")]
mod windows;

/// Manage a virtual machine for running the PS4 processes.
///
/// Do not create more than one Hypervisor because it will not work on macOS.
pub struct Hypervisor {
    platform: P,
    ram: Arc<Ram>,
    created_cpu: AtomicUsize,
}

impl Hypervisor {
    pub const VCPU: usize = 8;

    pub fn new() -> Result<Self, HypervisorError> {
        let ram = Arc::new(Ram::new(0).map_err(HypervisorError::CreateRamFailed)?);
        let platform = Self::setup_platform(Self::VCPU, ram.clone())?;

        Ok(Self {
            platform,
            ram,
            created_cpu: AtomicUsize::new(0),
        })
    }

    /// This method must be called by the thread that is going to drive the created CPU.
    ///
    /// # Panics
    /// If called more than [`Self::VCPU`].
    pub fn create_cpu(&self) -> Result<<P as Platform>::Cpu<'_>, <P as Platform>::CpuErr> {
        let id = self.created_cpu.fetch_add(1, Ordering::Relaxed);

        if id >= Self::VCPU {
            panic!("create_cpu cannot called more than {} times", Self::VCPU);
        }

        self.platform.create_cpu(id)
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
}

#[cfg(target_os = "linux")]
type P = self::linux::Kvm;

#[cfg(target_os = "windows")]
type P = self::windows::Whp;

#[cfg(target_os = "macos")]
type P = self::macos::Hf;

/// Object that has a physical address in the virtual machine.
pub trait MemoryAddr {
    /// Physical address in the virtual machine.
    fn vm_addr(&self) -> usize;

    /// Address in our process.
    fn host_addr(&self) -> *mut ();

    /// Total size of the object, in bytes.
    fn len(&self) -> usize;
}

/// Represents an error when [`Hypervisor::new()`] fails.
#[derive(Debug, Error)]
pub enum HypervisorError {
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
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(std::num::NonZero<std::ffi::c_int>),

    #[cfg(target_os = "macos")]
    #[error("couldn't map memory to the VM")]
    MapRamFailed(std::num::NonZero<std::ffi::c_int>),
}
