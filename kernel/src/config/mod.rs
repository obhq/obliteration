pub use self::dipsw::*;

use alloc::boxed::Box;
use alloc::sync::Arc;
use config::{ConsoleId, ProductId, QaFlags};
use core::num::NonZero;
use macros::elf_note;

mod dipsw;

pub const PAGE_SHIFT: usize = 14; // 16K
pub const PAGE_SIZE: NonZero<usize> = NonZero::new(1 << PAGE_SHIFT).unwrap();
pub const PAGE_MASK: NonZero<usize> = NonZero::new(PAGE_SIZE.get() - 1).unwrap();

/// Runtime configurations for the kernel populated from [`config::Config`].
pub struct Config {
    max_cpu: NonZero<usize>,
    unknown_dmem1: u8, // TODO: Figure out a correct name.
    idps: &'static ConsoleId,
    qa: bool,
    qa_flags: &'static QaFlags,
    env_vars: Box<[(&'static str, &'static str)]>, // kenvp
}

impl Config {
    pub fn new(src: &'static ::config::Config) -> Arc<Self> {
        let env_vars = Self::load_env(src);

        Arc::new(Self {
            max_cpu: src.max_cpu,
            unknown_dmem1: 0,
            idps: &src.idps,
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

    pub fn idps(&self) -> &'static ConsoleId {
        self.idps
    }

    /// See `getenv` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x39D0A0|
    pub fn env(&self, name: &str) -> Option<&'static str> {
        for &(k, v) in &self.env_vars {
            if k == name {
                return Some(v);
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

    /// See `sceSblAIMgrIsDevKit` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x078F50|
    pub fn is_devkit(&self) -> bool {
        self.idps.product == ProductId::DEVKIT
    }

    /// See `sceSblAIMgrIsTestKit` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x0790A0|
    pub fn is_testkit(&self) -> bool {
        self.idps.product == ProductId::TESTKIT
    }

    /// See `sceKernelCheckDipsw` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x654D70|
    pub fn dipsw(&self, _: Dipsw) -> bool {
        if !self.is_testkit() {
            if !self.is_devkit() {
                return false;
            }
        } else {
            todo!()
        }

        todo!()
    }

    /// See `init_dynamic_kenv` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x39DC90|
    fn load_env(config: &'static ::config::Config) -> Box<[(&'static str, &'static str)]> {
        config.env().collect()
    }
}

#[elf_note(section = ".note.obkrnl.page-size", name = "obkrnl", ty = 0)]
static NOTE_PAGE_SIZE: [u8; size_of::<usize>()] = PAGE_SIZE.get().to_ne_bytes();
