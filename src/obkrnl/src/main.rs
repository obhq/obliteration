#![no_std]
#![cfg_attr(not(test), no_main)]

use crate::config::set_boot_env;
use core::arch::asm;
use core::panic::PanicInfo;
use obconf::BootEnv;

mod config;
mod console;
mod imgfmt;

/// Entry point of the kernel.
///
/// This will be called by a bootloader or a hypervisor. The following are requirements before
/// transfer a control to this function:
///
/// 1. The kernel does not remap itself so it must be mapped at a desired virtual address and all
///    relocations must be applied.
///
/// See PS4 kernel entry point for a reference.
#[allow(dead_code)]
#[cfg_attr(not(test), no_mangle)]
extern "C" fn _start(env: &'static BootEnv) -> ! {
    // SAFETY: This is safe because we called it as the first thing here.
    unsafe { set_boot_env(env) };

    info!("Starting Obliteration Kernel.");

    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            asm!("hlt")
        };
        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!("wfi")
        };
    }
}

#[allow(dead_code)]
#[cfg_attr(not(test), panic_handler)]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
