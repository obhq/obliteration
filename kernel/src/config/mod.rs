pub use self::arch::*;
pub use self::dipsw::*;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use config::QaFlags;
use core::num::NonZero;
use krt::warn;
use macros::elf_note;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod dipsw;

pub const PAGE_SIZE: NonZero<usize> = NonZero::new(1 << PAGE_SHIFT).unwrap();
pub const PAGE_MASK: NonZero<usize> = NonZero::new(PAGE_SIZE.get() - 1).unwrap();

/// Runtime configurations for the kernel populated from [`config::Config`].
pub struct Config {
    max_cpu: NonZero<usize>,
    unknown_dmem1: u8, // TODO: Figure out a correct name.
    qa: bool,
    qa_flags: &'static QaFlags,
    env_vars: Box<[&'static str]>, // kenvp
}

impl Config {
    pub fn new(src: &'static ::config::Config) -> Arc<Self> {
        let env_vars = Self::load_env(src);

        Arc::new(Self {
            max_cpu: src.max_cpu,
            unknown_dmem1: 0,
            qa: src.qa,
            qa_flags: &src.qa_flags,
            env_vars,
        })
    }

    pub fn max_cpu(&self) -> NonZero<usize> {
        self.max_cpu
    }

    pub fn unknown_dmem1(&self) -> u8 {
        self.unknown_dmem1
    }

    /// See `getenv` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x39D0A0|
    pub fn env(&self, name: &str) -> Option<&'static str> {
        for &v in &self.env_vars {
            // Check prefix.
            let v = match v.strip_prefix(name) {
                Some(v) => v,
                None => continue,
            };

            // Check if '=' follow the name.
            let mut iter = v.chars();

            if iter.next().is_some_and(|c| c == '=') {
                return Some(iter.as_str());
            }
        }

        None
    }

    /// See `sceSblRcMgrIsAllowDisablingAslr` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3CA8F0|
    pub fn is_allow_disabling_aslr(&self) -> bool {
        self.qa && self.qa_flags.internal_dev()
    }

    /// See `sceKernelCheckDipsw` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x654D70|
    pub fn dipsw(&self, _: Dipsw) -> bool {
        todo!()
    }

    /// See `init_dynamic_kenv` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x39DC90|
    fn load_env(config: &'static ::config::Config) -> Box<[&'static str]> {
        // Our implementation a bit different here. On Orbis they required the last entry to be an
        // empty string but we don't.
        let mut list = Vec::with_capacity(0x1000);
        let mut rem = config.env_vars.as_slice();
        let mut n = -1;

        while !rem.is_empty() {
            // We don't use https://crates.io/crates/memchr because it is likely to be useless since
            // we don't have access to SIMD instructions in the kernel.
            let v = match rem.iter().position(|&b| b == 0) {
                Some(i) => {
                    let v = &rem[..i];
                    rem = &rem[(i + 1)..];
                    v
                }
                None => core::mem::replace(&mut rem, b""),
            };

            // On Orbis they allow the first entry to be an empty string but I don't think it is
            // intended behavior.
            if v.is_empty() {
                break;
            }

            n += 1;

            // We required string to be UTF-8 while the Orbis does not.
            let v = match core::str::from_utf8(v) {
                Ok(v) => v,
                Err(_) => {
                    warn!("Ignoring non-UTF-8 kenv string #{n}.");
                    continue;
                }
            };

            if (v.len() + 1) >= 259 {
                warn!("Too long kenv string, ignoring {v}.");
                continue;
            } else if list.len() > 511 {
                warn!("Too many kenv strings, ignoring {v}.");
                continue;
            }

            list.push(v);
        }

        list.into_boxed_slice()
    }
}

#[elf_note(section = ".note.obkrnl.page-size", name = "obkrnl", ty = 0)]
static NOTE_PAGE_SIZE: [u8; size_of::<usize>()] = PAGE_SIZE.get().to_ne_bytes();
