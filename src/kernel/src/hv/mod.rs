#[cfg(target_os = "macos")]
use std::num::NonZero;
use thiserror::Error;

pub use self::ram::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod mac;

mod ram;

#[cfg(target_os = "windows")]
mod win32;

/// Manage a virtual machine for running the PS4 processes.
///
/// Do not create more than one Hypervisor because it will not work on macOS.
pub struct Hypervisor {
    #[cfg(target_os = "linux")]
    vcpus: self::linux::VCpus, // Drop before VM.
    #[cfg(target_os = "linux")]
    vm: self::linux::Vm, // Drop before KVM.
    #[cfg(target_os = "linux")]
    kvm: self::linux::Kvm,

    #[cfg(target_os = "windows")]
    part: self::win32::Partition,

    #[cfg(target_os = "macos")]
    vm: self::mac::Vm,

    ram: Ram, // Drop after a VM.
}

impl Hypervisor {
    pub fn new() -> Result<Self, HypervisorError> {
        let ram = Ram::new(0).map_err(HypervisorError::CreateRamFailed)?;

        // Initialize platform hypervisor.
        #[cfg(target_os = "linux")]
        return Self::new_linux(ram);

        #[cfg(target_os = "windows")]
        return Self::new_windows(ram);

        #[cfg(target_os = "macos")]
        return Self::new_mac(ram);
    }

    #[cfg(target_os = "linux")]
    fn new_linux(ram: Ram) -> Result<Self, HypervisorError> {
        // Open KVM device.
        let kvm = self::linux::Kvm::open()?;

        if kvm.max_vcpus()? < 8 {
            return Err(HypervisorError::MaxCpuTooLow);
        }

        // Create a new VM.
        let vm = kvm.create_vm()?;

        vm.set_user_memory_region(
            0,
            ram.vm_addr().try_into().unwrap(),
            ram.len().try_into().unwrap(),
            ram.host_addr().cast(),
        )?;

        let mmap_size = kvm.get_vcpu_mmap_size()?;
        let vcpus = vm
            .create_vcpus(mmap_size)
            .map_err(HypervisorError::CreateVCpusError)?;

        Ok(Self {
            vcpus,
            vm,
            kvm,
            ram,
        })
    }

    #[cfg(target_os = "windows")]
    fn new_windows(ram: Ram) -> Result<Self, HypervisorError> {
        // Setup a partition.
        let mut part =
            self::win32::Partition::new().map_err(HypervisorError::CreatePartitionFailed)?;

        part.set_vcpu(8)
            .map_err(HypervisorError::SetCpuCountFailed)?;
        part.setup()
            .map_err(HypervisorError::SetupPartitionFailed)?;

        // Map memory.
        part.map_gpa(
            ram.host_addr().cast(),
            ram.vm_addr().try_into().unwrap(),
            ram.len().try_into().unwrap(),
        )
        .map_err(HypervisorError::MapRamFailed)?;

        Ok(Self { part, ram })
    }

    #[cfg(target_os = "macos")]
    fn new_mac(ram: Ram) -> Result<Self, HypervisorError> {
        // Create a VM.
        let vm = self::mac::Vm::new().map_err(HypervisorError::CreateVmFailed)?;

        if vm.capability(0).map_err(HypervisorError::GetMaxCpuFailed)? < 8 {
            return Err(HypervisorError::MaxCpuTooLow);
        }

        // Map memory.
        vm.vm_map(
            ram.host_addr().cast(),
            ram.vm_addr().try_into().unwrap(),
            ram.len(),
        )
        .map_err(HypervisorError::MapRamFailed)?;

        Ok(Self { vm, ram })
    }
}

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

    #[cfg(target_os = "linux")]
    #[error("couldn't create vCPUs")]
    CreateVCpusError(#[source] self::linux::CreateVCpusError),

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
    CreateVmFailed(NonZero<std::ffi::c_int>),

    #[cfg(target_os = "macos")]
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(NonZero<std::ffi::c_int>),

    #[cfg(target_os = "macos")]
    #[error("couldn't map memory to the VM")]
    MapRamFailed(NonZero<std::ffi::c_int>),
}
