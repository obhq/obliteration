pub use self::ram::*;
use thiserror::Error;

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
    vcpus: [std::os::fd::OwnedFd; 8], // Drop before VM.
    #[cfg(target_os = "linux")]
    vm: std::os::fd::OwnedFd, // Drop before KVM.
    #[cfg(target_os = "linux")]
    kvm: std::os::fd::OwnedFd,

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
        use std::os::fd::AsFd;

        // Open KVM device.
        let kvm = self::linux::open_kvm()?;

        if self::linux::max_vcpus(kvm.as_fd()).map_err(HypervisorError::GetMaxCpuFailed)? < 8 {
            return Err(HypervisorError::MaxCpuTooLow);
        }

        // Create a new VM.
        let vm = self::linux::create_vm(kvm.as_fd()).map_err(HypervisorError::CreateVmFailed)?;

        self::linux::set_user_memory_region(
            vm.as_fd(),
            0,
            ram.vm_addr().try_into().unwrap(),
            ram.len().try_into().unwrap(),
            ram.host_addr().cast(),
        )
        .map_err(HypervisorError::MapRamFailed)?;

        let vcpus = [
            self::linux::create_vcpu(vm.as_fd(), 0)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 0))?,
            self::linux::create_vcpu(vm.as_fd(), 1)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 1))?,
            self::linux::create_vcpu(vm.as_fd(), 2)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 2))?,
            self::linux::create_vcpu(vm.as_fd(), 3)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 3))?,
            self::linux::create_vcpu(vm.as_fd(), 4)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 4))?,
            self::linux::create_vcpu(vm.as_fd(), 5)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 5))?,
            self::linux::create_vcpu(vm.as_fd(), 6)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 6))?,
            self::linux::create_vcpu(vm.as_fd(), 7)
                .map_err(|e| HypervisorError::CreateVcpuFailed(e, 7))?,
        ];

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
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't create a VM")]
    CreateVmFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't map the RAM to the VM")]
    MapRamFailed(#[source] std::io::Error),

    #[cfg(target_os = "linux")]
    #[error("couldn't create vcpu #{1}")]
    CreateVcpuFailed(#[source] std::io::Error, u8),

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
    CreateVmFailed(std::ffi::c_int),

    #[cfg(target_os = "macos")]
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(std::ffi::c_int),

    #[cfg(target_os = "macos")]
    #[error("couldn't map memory to the VM")]
    MapRamFailed(std::ffi::c_int),
}
