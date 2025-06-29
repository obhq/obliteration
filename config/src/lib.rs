#![no_std]

pub use self::env::*;
pub use self::idps::*;
pub use self::qa::*;

use core::num::NonZero;

mod env;
mod idps;
mod qa;

/// Contains information how the kernel is mapped.
#[repr(C)]
pub struct KernelMap {
    /// Virtual address of the kernel.
    ///
    /// This must be the address of ELF header of the kernel.
    pub kern_vaddr: usize,
    /// The beginning of free virtual address.
    ///
    /// All address after this must not contains any data.
    pub free_vaddr: NonZero<usize>,
}

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
    pub idps: ConsoleId,
    pub qa: bool,
    pub qa_flags: QaFlags,
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub env_vars: [u8; 132096], // See init_dynamic_kenv() on the Orbis for this number.
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_cpu: NonZero::new(1).unwrap(),
            idps: ConsoleId::default(),
            qa: false,
            qa_flags: QaFlags::default(),
            env_vars: [0; 132096],
        }
    }
}
