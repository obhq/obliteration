use core::arch::asm;

/// Perform panic after printing the panic message.
pub fn panic() -> ! {
    loop {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!("wfi")
        };
        #[cfg(target_arch = "x86_64")]
        unsafe {
            asm!("hlt")
        };
    }
}
