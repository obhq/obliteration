use super::{MapError, Memory};
use crate::memory::MemoryManager;
use elf::Elf;
use std::fs::File;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.h#L147.
pub struct Module<'a> {
    id: u32,
    entry: Option<usize>,
    tls_index: u32,
    proc_param: Option<(usize, usize)>,
    image: Elf<File>,
    memory: Memory<'a>,
}

impl<'a> Module<'a> {
    pub(super) fn map(
        mm: &'a MemoryManager,
        mut image: Elf<File>,
        base: usize,
        id: u32,
        tls_index: u32,
    ) -> Result<Self, MapError> {
        // Map the image to the memory.
        let mut memory = Memory::new(mm, &image, base)?;

        memory.load(|prog, buf| {
            image
                .read_program(prog, buf)
                .map_err(|e| MapError::ReadProgramFailed(prog, e))
        })?;

        // Apply memory protection.
        if let Err(e) = memory.protect() {
            return Err(MapError::ProtectMemoryFailed(e));
        }

        Ok(Self {
            id,
            entry: image.entry_addr().map(|v| base + v),
            tls_index,
            proc_param: image.proc_param().map(|i| {
                let p = image.programs().get(i).unwrap();
                (base + p.addr(), p.file_size().try_into().unwrap())
            }),
            image,
            memory,
        })
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn entry(&self) -> Option<usize> {
        self.entry
    }

    pub fn tls_index(&self) -> u32 {
        self.tls_index
    }

    pub fn proc_param(&self) -> Option<&(usize, usize)> {
        self.proc_param.as_ref()
    }

    pub fn image(&self) -> &Elf<File> {
        &self.image
    }

    pub fn memory(&self) -> &Memory<'a> {
        &self.memory
    }
}
