use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[cfg(target_os = "macos")]
mod darwin;

/// Manage a virtual machine of the current process.
///
/// Each process can have only one VM. The reason this type is not a global variable is because we
/// want to be able to drop it.
pub struct Hypervisor {
    #[allow(dead_code)]
    active: Active, // Drop as the last one.
}

impl Hypervisor {
    pub fn new() -> Result<Self, NewError> {
        let active = Active::new().ok_or(NewError::Active)?;

        #[cfg(target_os = "macos")]
        match unsafe { self::darwin::hv_vm_create(std::ptr::null_mut()) } {
            0 => {}
            v => return Err(NewError::HostFailed(v)),
        }

        Ok(Self { active })
    }
}

impl Drop for Hypervisor {
    #[cfg(target_os = "linux")]
    fn drop(&mut self) {}

    #[cfg(target_os = "windows")]
    fn drop(&mut self) {}

    #[cfg(target_os = "macos")]
    fn drop(&mut self) {
        let status = unsafe { self::darwin::hv_vm_destroy() };

        if status != 0 {
            panic!("hv_vm_destroy() was failed with {status:#x}");
        }
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

/// Represents an error when [`Hypervisor::new()`] was failed.
#[derive(Debug, Error)]
pub enum NewError {
    #[error("there is an active hypervisor")]
    Active,

    #[cfg(target_os = "macos")]
    #[error("the host failed to create the hypervisor ({0:#x})")]
    HostFailed(std::ffi::c_int),
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
