use core::ptr::null;
use macros::elf_note;
use obconf::{BootEnv, Config};

pub use self::arch::*;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

/// # Interupt safety
/// This function is interupt safe.
pub fn boot_env() -> &'static BootEnv {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { &*BOOT_ENV }
}

/// # Interupt safety
/// This function is interupt safe.
pub fn config() -> &'static Config {
    // SAFETY: This is safe because the setup() requirements.
    unsafe { &*CONFIG }
}

/// # Safety
/// This function must be called immediately in the kernel entry point. After that it must never
/// be called again.
pub unsafe fn setup(env: &'static BootEnv, conf: &'static Config) {
    BOOT_ENV = env;
    CONFIG = conf;
}

static mut BOOT_ENV: *const BootEnv = null();
static mut CONFIG: *const Config = null();

#[elf_note(section = ".note.obkrnl.page-size", name = "obkrnl", ty = 0)]
static NOTE_PAGE_SIZE: [u8; size_of::<usize>()] = PAGE_SIZE.to_ne_bytes();
