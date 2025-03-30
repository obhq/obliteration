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
#[cfg_attr(feature = "serde", serde(default))]
pub struct Config {
    pub max_cpu: NonZero<usize>,
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub env_vars: [u8; 132096], // See init_dynamic_kenv() on the Orbis for this number.
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_cpu: NonZero::new(1).unwrap(),
            env_vars: [0; 132096],
        }
    }
}
