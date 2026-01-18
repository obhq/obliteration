use config::{BootEnv, KernelMap};
use core::ptr::null;

pub fn phys_vaddr() -> usize {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { PHYS_VADDR }
}

pub fn boot_env() -> &'static BootEnv {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { &*BOOT_ENV }
}

/// # Safety
/// This function must be called immediately in the [_start](super::_start) function. After that it
/// must never be called again.
#[allow(dead_code)]
pub(super) unsafe fn setup(map: &'static KernelMap, env: &'static BootEnv) {
    unsafe { PHYS_VADDR = map.phys_vaddr };
    unsafe { BOOT_ENV = env };
}

static mut PHYS_VADDR: usize = 0;
static mut BOOT_ENV: *const BootEnv = null();
