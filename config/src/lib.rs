#![no_std]

pub use self::env::*;
pub use self::idps::*;
pub use self::qa::*;

use core::iter::Peekable;
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
    /// Size of virtual address the kernel is mapped.
    ///
    /// This include everything that need to be lived forever (e.g. stack for main CPU).
    pub kern_vsize: NonZero<usize>,
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

impl Config {
    /// # Panics
    /// If no zero in [`Self::env_vars`]. This only happens if you construct [`Config`] manually or
    /// modify [`Self::env_vars`] directly.
    pub fn extend_env<I, K, V>(&mut self, vars: I) -> Option<Peekable<I::IntoIter>>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let end = self.env_vars.iter().position(|&b| b == 0).unwrap();
        let begin = if end == 0 { 0 } else { end + 1 };
        let mut space = &mut self.env_vars[begin..];
        let mut iter = vars.into_iter().peekable();

        while let Some((k, v)) = iter.peek() {
            // Check if available space is enough.
            let k = k.as_ref();
            let v = v.as_ref();
            let l = k.len() + 1 + v.len() + 1;
            let b = match space.get_mut(..l) {
                Some(v) => v,
                None => return Some(iter),
            };

            // Write the entry then advance the iterator.
            b[..k.len()].copy_from_slice(k.as_bytes());
            b[k.len()] = b'=';
            b[(k.len() + 1)..(l - 1)].copy_from_slice(v.as_bytes());
            b[l - 1] = 0;

            space = &mut space[l..];
            iter.next();
        }

        None
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut c = Self {
            max_cpu: NonZero::new(1).unwrap(),
            idps: ConsoleId::default(),
            qa: false,
            qa_flags: QaFlags::default(),
            env_vars: [0; 132096],
        };

        // TODO: Verify if this variables is zero on Orbis.
        c.extend_env([("hw.memtest.tests", "0")]);
        c
    }
}
