use crate::config::boot_env;
use obconf::BootEnv;

mod vm;

/// Perform panic after printing the panic message.
///
/// # Context safety
/// This function does not require a CPU context.
///
/// # Interupt safety
/// This function is interupt safe.
pub fn panic() -> ! {
    match boot_env() {
        BootEnv::Vm(env) => self::vm::panic(env),
    }
}
