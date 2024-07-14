#![no_std]
#![no_main]

#[no_mangle]
fn _start() -> ! {
    loop {
        unsafe { core::arch::x86_64::_mm_pause() };
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
