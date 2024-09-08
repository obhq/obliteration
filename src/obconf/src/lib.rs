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
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Config {
    pub max_cpu: NonZero<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_cpu: NonZero::new(1).unwrap(),
        }
    }
}
