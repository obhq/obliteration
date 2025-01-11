use config::{BootEnv, Config};
use core::ptr::null;

pub fn boot_env() -> &'static BootEnv {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { &*BOOT_ENV }
}

pub fn config() -> &'static Config {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { &*CONFIG }
}

/// # Safety
/// This function must be called immediately in the [_start](super::_start) function. After that it
/// must never be called again.
pub(super) unsafe fn setup(env: &'static BootEnv, conf: &'static Config) {
    BOOT_ENV = env;
    CONFIG = conf;
}

static mut BOOT_ENV: *const BootEnv = null();
static mut CONFIG: *const Config = null();
