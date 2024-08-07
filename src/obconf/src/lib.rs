#![no_std]

pub use self::env::*;

mod env;

/// Information about the boot environment.
#[repr(C)]
pub enum BootEnv {
    Vm(Vm),
}
