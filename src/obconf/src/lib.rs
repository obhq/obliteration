#![no_std]

use core::num::NonZero;

pub use self::env::*;

mod env;

/// Contains information about the boot environment.
#[repr(C)]
pub enum BootEnv {
    Vm(Vm),
}

/// Runtime configurations for the kernel.
#[repr(C)]
pub struct Config {
    pub max_cpu: NonZero<usize>,
}
