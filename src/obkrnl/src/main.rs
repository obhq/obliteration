#![no_std]
#![no_main]

#[no_mangle]
fn _start() -> ! {
    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
