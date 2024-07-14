#![no_std]
#![cfg_attr(not(test), no_main)]

use core::panic::PanicInfo;

#[cfg_attr(not(test), no_mangle)]
fn _start() -> ! {
    loop {
        unsafe { core::arch::x86_64::_mm_pause() };
    }
}

#[allow(dead_code)]
#[cfg_attr(not(test), panic_handler)]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
