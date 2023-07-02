pub use mem::*;
pub use module::*;

use crate::fs::path::VPathBuf;
use crate::fs::Fs;
use crate::memory::{MemoryManager, MmapError, MprotectError};
use elf::{Elf, FileType, ReadProgramError};
use std::fs::File;
use std::sync::Arc;
use thiserror::Error;

mod mem;
mod module;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.c.
pub struct RuntimeLinker<'a> {
    app: Arc<Module<'a>>, // obj_main
}

impl<'a> RuntimeLinker<'a> {
    pub fn new(fs: &'a Fs, mm: &'a MemoryManager) -> Result<Self, RuntimeLinkerError> {
        // Get path to eboot.bin.
        let mut path = fs.app().join("app0").unwrap();

        path.push("eboot.bin").unwrap();

        // Get eboot.bin.
        let file = match fs.get_file(&path) {
            Some(v) => v,
            None => return Err(RuntimeLinkerError::FileNotFound(path)),
        };

        // Open eboot.bin.
        let elf = match File::open(file.path()) {
            Ok(v) => match Elf::open(file.virtual_path(), v) {
                Ok(v) => v,
                Err(e) => return Err(RuntimeLinkerError::OpenElfFailed(file.into_vpath(), e)),
            },
            Err(e) => return Err(RuntimeLinkerError::OpenFileFailed(file.into_vpath(), e)),
        };

        // Get base address.
        let base = if elf.ty() == FileType::ET_SCE_DYNEXEC {
            if elf.dynamic_linking().is_none() {
                return Err(RuntimeLinkerError::InvalidElf(file.into_vpath()));
            }

            0x400000
        } else {
            0
        };

        // Load eboot.bin.
        let app = match Module::load(mm, elf, base) {
            Ok(v) => Arc::new(v),
            Err(e) => return Err(RuntimeLinkerError::LoadFailed(file.into_vpath(), e)),
        };

        Ok(Self { app })
    }

    pub fn app(&self) -> &Arc<Module<'a>> {
        &self.app
    }

    pub fn kernel(&self) -> Option<&Arc<Module<'a>>> {
        None
    }

    pub fn for_each<F, E>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&Arc<Module<'a>>) -> Result<(), E>,
    {
        f(&self.app)?;
        Ok(())
    }
}

/// Represents the error for [`RuntimeLinker`] initialization.
#[derive(Debug, Error)]
pub enum RuntimeLinkerError {
    #[error("{0} does not exists")]
    FileNotFound(VPathBuf),

    #[error("cannot open {0}")]
    OpenFileFailed(VPathBuf, #[source] std::io::Error),

    #[error("cannot open {0}")]
    OpenElfFailed(VPathBuf, #[source] elf::OpenError),

    #[error("{0} is not a valid (S)ELF")]
    InvalidElf(VPathBuf),

    #[error("cannot load {0}")]
    LoadFailed(VPathBuf, #[source] LoadError),
}

/// Represents an error for (S)ELF loading.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] MmapError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] ReadProgramError),

    #[error("cannot protect the memory")]
    ProtectMemoryFailed(#[source] MprotectError),
}
