#![no_std]

pub use self::env::*;
pub use self::idps::*;
pub use self::qa::*;

use core::iter::{FusedIterator, Peekable};
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
    env_vars: [u8; 132096], // See init_dynamic_kenv() on the Orbis for this number.
}

impl Config {
    const ENV_END: u8 = 0xF8; // Invalid first byte of UTF-8.
    const END_SEP: u8 = 0xF9; // Same here.

    /// # Panics
    /// [`Iterator::next()`] on the returned iterator will panic if this config is corrupted. This
    /// only happens if you load this config with Serde and the serialized data is corrupted.
    pub fn env(&self) -> impl Iterator<Item = (&str, &str)> {
        EnvIter {
            rem: self.env_vars.as_slice(),
        }
    }

    /// # Panics
    /// This config is corrupted. This only happens if you load this config with Serde and the
    /// serialized data is corrupted.
    pub fn extend_env<I, K, V>(&mut self, vars: I) -> Option<Peekable<I::IntoIter>>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let end = self
            .env_vars
            .iter()
            .position(|&b| b == Self::ENV_END)
            .unwrap();
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
            b[k.len()] = Self::END_SEP; // Orbis use '=' as a separator.
            b[(k.len() + 1)..(l - 1)].copy_from_slice(v.as_bytes());
            b[l - 1] = Self::ENV_END;

            space = &mut space[l..];

            if let Some(v) = space.first_mut() {
                *v = Self::ENV_END;
            }

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
            env_vars: [Self::ENV_END; 132096], // Orbis fill this with NULs.
        };

        // TODO: Verify if this variables is zero on Orbis.
        c.extend_env([("hw.memtest.tests", "0")]);
        c
    }
}

/// Implementation of [`Iterator`] for [`Config::env_vars`].
struct EnvIter<'a> {
    rem: &'a [u8],
}

impl<'a> Iterator for EnvIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        // This basically an implementation of init_dynamic_kenv + getenv. Note that we don't limit
        // the number of entires and the length of each entry.
        if self.rem.is_empty() {
            return None;
        }

        // We don't use https://crates.io/crates/memchr because it is likely to be useless since we
        // don't have access to SIMD instructions in the kernel.
        let i = self.rem.iter().position(|&b| b == Config::ENV_END).unwrap();

        // On Orbis they allow the first entry to be an empty string but I don't think it is
        // intended behavior.
        let (v, r) = self.rem.split_at(i);

        if v.is_empty() {
            return None;
        }

        self.rem = &r[1..];

        // We required string to be UTF-8 while the Orbis does not.
        let i = v.iter().position(|&b| b == Config::END_SEP).unwrap();
        let (k, v) = v.split_at(i);
        let k = core::str::from_utf8(k).unwrap();
        let v = core::str::from_utf8(&v[1..]).unwrap();

        Some((k, v))
    }
}

impl<'a> FusedIterator for EnvIter<'a> {}
