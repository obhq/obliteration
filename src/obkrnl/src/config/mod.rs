use core::ptr::null;
use macros::elf_note;
use obconf::BootEnv;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;

#[cfg(target_arch = "x86_64")]
mod x86_64;

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

#[elf_note(section = ".note.obkrnl.page-size", name = "obkrnl", ty = 0)]
static NOTE_PAGE_SIZE: [u8; size_of::<usize>()] = PAGE_SIZE.to_ne_bytes();
