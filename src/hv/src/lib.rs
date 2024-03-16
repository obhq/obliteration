use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[cfg(target_os = "macos")]
mod darwin;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(target_os = "windows")]
mod win32;

/// Manage a virtual machine of the current process.
///
/// Each process can have only one VM. The reason this type is not a global variable is because we
/// want to be able to drop it.
pub struct Hypervisor {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    vm: std::os::fd::OwnedFd, // Drop before KVM.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    kvm: std::os::fd::OwnedFd,
    #[cfg(target_os = "windows")]
    whp: self::win32::Partition,
    #[cfg(target_os = "macos")]
    vm: self::darwin::Vm,
    #[allow(dead_code)]
    active: Active, // Drop as the last one.
}

impl Hypervisor {
    /// # Safety
    /// `ram` cannot be null and must be allocated with a Virtual Memory API (e.g. `mmap` on *nix or
    /// `VirtualAlloc` on Windows). This memory must be valid throughout the lifetime of the VM.
    pub unsafe fn new(
        cpu: NonZeroUsize,
        ram: *mut u8,
        addr: usize,
        len: NonZeroUsize,
    ) -> Result<Self, NewError> {
        // Check if another instance already active.
        let active = Active::new().ok_or(NewError::Active)?;

        // Make sure memory size is valid.
        let host_page_size = match Self::host_page_size() {
            #[cfg(unix)]
            Ok(v) => v,
            #[cfg(unix)]
            Err(e) => return Err(NewError::GetHostPageSizeFailed(e)),
            #[cfg(windows)]
            v => v,
        };

        if len.get() % host_page_size != 0 {
            return Err(NewError::InvalidMemorySize);
        }

        // Initialize platform hypervisor.
        #[cfg(any(target_os = "linux", target_os = "android"))]
        return Self::new_linux(active, cpu, ram, addr, len.get());

        #[cfg(target_os = "windows")]
        return Self::new_windows(active, cpu, ram, addr, len.get());

        #[cfg(target_os = "macos")]
        return Self::new_mac(active, cpu, ram, addr, len.get());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn new_linux(
        active: Active,
        cpu: NonZeroUsize,
        ram: *mut u8,
        addr: usize,
        len: usize,
    ) -> Result<Self, NewError> {
        use std::os::fd::AsFd;

        // Open KVM device.
        let kvm = self::linux::open_kvm()?;

        if cpu.get() > self::linux::max_vcpus(kvm.as_fd()).map_err(NewError::GetMaxCpuFailed)? {
            return Err(NewError::InvalidCpuCount);
        }

        // Create a new VM.
        let vm = self::linux::create_vm(kvm.as_fd()).map_err(NewError::CreateVmFailed)?;

        Ok(Self { vm, kvm, active })
    }

    #[cfg(target_os = "windows")]
    unsafe fn new_windows(
        active: Active,
        cpu: NonZeroUsize,
        ram: *mut u8,
        addr: usize,
        len: usize,
    ) -> Result<Self, NewError> {
        let mut whp = self::win32::Partition::new(cpu)?;

        whp.setup()?;

        Ok(Self { whp, active })
    }

    #[cfg(target_os = "macos")]
    unsafe fn new_mac(
        active: Active,
        cpu: NonZeroUsize,
        ram: *mut u8,
        addr: usize,
        len: usize,
    ) -> Result<Self, NewError> {
        // Create a VM.
        let vm = self::darwin::Vm::new()?;
        let cpu: u64 = cpu.get().try_into().unwrap();

        if cpu > vm.capability(0).map_err(NewError::GetMaxCpuFailed)? {
            return Err(NewError::InvalidCpuCount);
        }

        Ok(Self { vm, active })
    }

    #[cfg(unix)]
    fn host_page_size() -> Result<usize, std::io::Error> {
        let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

        if v < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(v.try_into().unwrap())
        }
    }

    #[cfg(windows)]
    fn host_page_size() -> usize {
        use windows_sys::Win32::System::SystemInformation::GetSystemInfo;

        let mut i = unsafe { std::mem::zeroed() };
        unsafe { GetSystemInfo(&mut i) };

        i.dwPageSize.try_into().unwrap()
    }
}

/// RAII object to set release ACTIVE.
struct Active;

impl Active {
    fn new() -> Option<Self> {
        ACTIVE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| Self)
            .ok()
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}

/// Represents an error when [`Hypervisor::new()`] fails.
#[derive(Debug, Error)]
pub enum NewError {
    #[error("there is an active hypervisor")]
    Active,

    #[cfg(unix)]
    #[error("couldn't determine page size of the host")]
    GetHostPageSizeFailed(#[source] std::io::Error),

    #[error("the number of CPU is not valid")]
    InvalidCpuCount,

    #[error("the specified memory size is not valid")]
    InvalidMemorySize,

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("couldn't open {0}")]
    OpenKvmFailed(&'static str, #[source] std::io::Error),

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("couldn't get KVM version")]
    GetKvmVersionFailed(#[source] std::io::Error),

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("unexpected KVM version")]
    KvmVersionMismatched,

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(#[source] std::io::Error),

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("couldn't create a VM")]
    CreateVmFailed(#[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't create WHP partition object ({0:#x})")]
    CreatePartitionFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "windows")]
    #[error("couldn't set number of CPU ({0:#x})")]
    SetCpuCountFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "windows")]
    #[error("couldn't setup WHP partition ({0:#x})")]
    SetupPartitionFailed(windows_sys::core::HRESULT),

    #[cfg(target_os = "macos")]
    #[error("couldn't create a VM ({0:#x})")]
    CreateVmFailed(std::ffi::c_int),

    #[cfg(target_os = "macos")]
    #[error("couldn't get maximum number of CPU for a VM")]
    GetMaxCpuFailed(std::ffi::c_int),
}

static ACTIVE: AtomicBool = AtomicBool::new(false);

// macOS requires additional entitlements for the application to use Hypervisor framework, which
// cannot be done with "cargo test".
#[cfg(not(target_os = "macos"))]
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error;

    #[test]
    fn new() {
        let cpu = unsafe { NonZeroUsize::new_unchecked(8) };
        let ram = Ram::new();

        unsafe { Hypervisor::new(cpu, ram.addr, 0, Ram::SIZE).unwrap() };
    }

    struct Ram {
        addr: *mut u8,
    }

    impl Ram {
        const SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1024 * 1024 * 1024 * 8) };

        #[cfg(unix)]
        fn new() -> Self {
            use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};
            use std::ptr::null_mut;

            let addr = unsafe {
                mmap(
                    null_mut(),
                    Self::SIZE.get(),
                    PROT_NONE,
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                )
            };

            if addr == MAP_FAILED {
                panic!("mmap() fails: {}", Error::last_os_error());
            }

            Self { addr: addr.cast() }
        }

        #[cfg(windows)]
        fn new() -> Self {
            use std::ptr::null;
            use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

            let addr =
                unsafe { VirtualAlloc(null(), Self::SIZE.get(), MEM_RESERVE, PAGE_NOACCESS) };

            if addr.is_null() {
                panic!("VirtualAlloc() fails: {}", Error::last_os_error());
            }

            Self { addr: addr.cast() }
        }
    }

    impl Drop for Ram {
        #[cfg(unix)]
        fn drop(&mut self) {
            use libc::munmap;

            if unsafe { munmap(self.addr.cast(), Self::SIZE.get()) } < 0 {
                panic!("munmap() fails: {}", Error::last_os_error());
            }
        }

        #[cfg(windows)]
        fn drop(&mut self) {
            use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

            if unsafe { VirtualFree(self.addr.cast(), 0, MEM_RELEASE) } == 0 {
                panic!("VirtualFree() fails: {}", Error::last_os_error());
            }
        }
    }
}
