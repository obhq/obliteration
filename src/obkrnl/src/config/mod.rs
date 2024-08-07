use core::ptr::null;
use obconf::BootEnv;

pub fn boot_env() -> &'static BootEnv {
    // SAFETY: This is safe because the set_boot_env() requirements.
    unsafe { &*BOOT_ENV }
}

/// # Safety
/// This function must be called immediately in the kernel entry point. After that it must never
/// be called again.
pub unsafe fn set_boot_env(env: &'static BootEnv) {
    BOOT_ENV = env;
}

static mut BOOT_ENV: *const BootEnv = null();
