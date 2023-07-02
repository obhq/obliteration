use super::{MapError, Memory};
use crate::memory::MemoryManager;
use elf::Elf;
use std::fs::File;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.h#L147.
pub struct Module<'a> {
    entry: Option<usize>,
    image: Elf<File>,
    memory: Memory<'a>,
}

impl<'a> Module<'a> {
    pub(super) fn map(
        mm: &'a MemoryManager,
        mut image: Elf<File>,
        base: usize,
    ) -> Result<Self, MapError> {
        // Map the image to the memory.
        let mut memory = Memory::new(mm, &image, base)?;

        memory.load(|prog, buf| {
            image
                .read_program(prog, buf)
                .map_err(|e| MapError::ReadProgramFailed(prog, e))
        })?;

        if let Err(e) = memory.protect() {
            return Err(MapError::ProtectMemoryFailed(e));
        }

        Ok(Self {
            entry: image.entry_addr().map(|v| base + v),
            image,
            memory,
        })
    }

    pub fn entry(&self) -> Option<usize> {
        self.entry
    }

    pub fn image(&self) -> &Elf<File> {
        &self.image
    }

    pub fn memory(&self) -> &Memory<'a> {
        &self.memory
    }
}
