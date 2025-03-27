use config::BootEnv;
use core::ptr::null;

pub fn boot_env() -> &'static BootEnv {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { &*BOOT_ENV }
}

/// # Safety
/// This function must be called immediately in the [_start](super::_start) function. After that it
/// must never be called again.
#[allow(dead_code)]
pub(super) unsafe fn setup(env: &'static BootEnv) {
    unsafe { BOOT_ENV = env };
}

static mut BOOT_ENV: *const BootEnv = null();
