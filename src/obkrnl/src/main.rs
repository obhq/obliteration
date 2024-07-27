#![no_std]
#![cfg_attr(not(test), no_main)]

use core::arch::asm;
use core::panic::PanicInfo;

/// See PS4 kernel entry point for a reference.
#[cfg_attr(not(test), no_mangle)]
fn _start() -> ! {
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
