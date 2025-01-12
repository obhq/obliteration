use crate::config::boot_env;
use config::BootEnv;

mod vm;

/// Perform panic after printing the panic message.
pub fn panic() -> ! {
    match boot_env() {
        BootEnv::Vm(env) => self::vm::panic(env),
    }
}
