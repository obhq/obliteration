use core::arch::asm;

/// Perform panic after printing the panic message.
///
/// # Interupt safety
/// This function is interupt safe.
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
