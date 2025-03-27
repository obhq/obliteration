pub use self::arch::*;

use alloc::sync::Arc;
use core::num::NonZero;
use macros::elf_note;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

pub const PAGE_SIZE: NonZero<usize> = NonZero::new(1 << PAGE_SHIFT).unwrap();
pub const PAGE_MASK: NonZero<usize> = NonZero::new(PAGE_SIZE.get() - 1).unwrap();

/// Runtime configurations for the kernel populated from [`config::Config`].
pub struct Config {
    max_cpu: NonZero<usize>,
}

impl Config {
    pub fn new(src: &'static ::config::Config) -> Arc<Self> {
        Arc::new(Self {
            max_cpu: src.max_cpu,
        })
    }

    pub fn max_cpu(&self) -> NonZero<usize> {
        self.max_cpu
    }

    pub fn env(&self, _: &str) -> Option<&'static str> {
        todo!()
    }
}

#[elf_note(section = ".note.obkrnl.page-size", name = "obkrnl", ty = 0)]
static NOTE_PAGE_SIZE: [u8; size_of::<usize>()] = PAGE_SIZE.get().to_ne_bytes();
