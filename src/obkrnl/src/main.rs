#![no_std]
#![cfg_attr(not(test), no_main)]

use core::panic::PanicInfo;

/// See PS4 kernel entry point for a reference.
#[cfg_attr(not(test), no_mangle)]
fn _start() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::x86_64::_mm_pause()
        };
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi")
        };
    }
}

#[allow(dead_code)]
#[cfg_attr(not(test), panic_handler)]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
