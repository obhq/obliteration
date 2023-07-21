pub use mem::*;
pub use module::*;

use crate::errno::{Errno, ENOEXEC};
use crate::fs::path::{VPath, VPathBuf};
use crate::fs::Fs;
use crate::memory::{MemoryManager, MmapError, MprotectError};
use elf::{Elf, FileInfo, FileType, ReadProgramError, Relocation};
use std::collections::VecDeque;
use std::fs::File;
use std::num::NonZeroI32;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use thiserror::Error;

mod mem;
mod module;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.c.
pub struct RuntimeLinker<'a> {
    fs: &'a Fs,
    mm: &'a MemoryManager,
    loaded: RwLock<VecDeque<Arc<Module<'a>>>>, // obj_list + obj_tail
    app: Arc<Module<'a>>,                      // obj_main
    kernel: Option<Arc<Module<'a>>>,           // obj_kernel
}

impl<'a> RuntimeLinker<'a> {
    pub fn new(fs: &'a Fs, mm: &'a MemoryManager) -> Result<Self, RuntimeLinkerError> {
        // Get path to eboot.bin.
        let mut path = fs.app().join("app0").unwrap();

        path.push("eboot.bin").unwrap();

        // Get eboot.bin.
        let file = match fs.get_file(&path) {
            Some(v) => v,
            None => return Err(RuntimeLinkerError::ExeNotFound(path)),
        };

        // Open eboot.bin.
        let elf = match File::open(file.path()) {
            Ok(v) => match Elf::open(file.virtual_path(), v) {
                Ok(v) => v,
                Err(e) => return Err(RuntimeLinkerError::OpenElfFailed(file.into_vpath(), e)),
            },
            Err(e) => return Err(RuntimeLinkerError::OpenExeFailed(file.into_vpath(), e)),
        };

        // Check image type.
        match elf.ty() {
            FileType::ET_EXEC | FileType::ET_SCE_EXEC | FileType::ET_SCE_REPLAY_EXEC => {
                if elf.dynamic_linking().is_none() {
                    todo!("A statically linked eboot.bin is not supported yet.");
                }
            }
            FileType::ET_SCE_DYNEXEC if elf.dynamic_linking().is_some() => {}
            _ => return Err(RuntimeLinkerError::InvalidExe(file.into_vpath())),
        }

        // Get base address.
        let base = if elf.ty() == FileType::ET_SCE_DYNEXEC {
            0x400000
        } else {
            0
        };

        // TODO: Apply remaining checks from exec_self_imgact.
        // Map eboot.bin.
        let app = match Module::map(mm, elf, base) {
            Ok(v) => Arc::new(v),
            Err(e) => return Err(RuntimeLinkerError::MapExeFailed(file.into_vpath(), e)),
        };

        Ok(Self {
            fs,
            mm,
            loaded: RwLock::new([app.clone()].into()),
            app,
            kernel: None,
        })
    }

    pub fn app(&self) -> &Arc<Module<'a>> {
        &self.app
    }

    pub fn kernel(&self) -> Option<&Arc<Module<'a>>> {
        self.kernel.as_ref()
    }

    pub fn for_each<F, E>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&Arc<Module<'a>>) -> Result<(), E>,
    {
        let loaded = self.loaded.read().unwrap();

        for m in loaded.deref() {
            f(m)?;
        }

        Ok(())
    }

    /// This method **ALWAYS** load the specified module without checking if the same module is
    /// already loaded.
    pub fn load(&self, path: &VPath) -> Result<Arc<Module<'a>>, LoadError> {
        // Get file.
        let file = match self.fs.get_file(path) {
            Some(v) => v,
            None => return Err(LoadError::FileNotFound),
        };

        // Open file.
        let elf = match File::open(file.path()) {
            Ok(v) => match Elf::open(file.virtual_path(), v) {
                Ok(v) => v,
                Err(e) => return Err(LoadError::OpenElfFailed(e)),
            },
            Err(e) => return Err(LoadError::OpenFileFailed(e)),
        };

        // Check image type.
        if elf.ty() != FileType::ET_SCE_DYNAMIC {
            return Err(LoadError::InvalidElf);
        }

        // TODO: Apply remaining checks from self_load_shared_object.
        // Map file.
        let module = match Module::map(self.mm, elf, 0) {
            Ok(v) => Arc::new(v),
            Err(e) => return Err(LoadError::MapFailed(e)),
        };

        // Load to loaded list.
        self.loaded.write().unwrap().push_back(module.clone());

        Ok(module)
    }

    /// # Safety
    /// No other threads may access the memory of all loaded modules.
    pub unsafe fn relocate(&self) -> Result<(), RelocateError> {
        // TODO: Check what the PS4 actually doing.
        let loaded = self.loaded.read().unwrap();

        for m in loaded.deref() {
            self.relocate_single(m)?;
        }

        Ok(())
    }

    pub fn set_kernel(&mut self, m: Arc<Module<'a>>) {
        self.kernel = Some(m);
    }

    /// See `relocate_one_object` on the PS4 kernel for a reference.
    unsafe fn relocate_single(&self, module: &Module<'a>) -> Result<(), RelocateError> {
        let image = module.image();
        let path: &VPath = image.name().try_into().unwrap();
        let info = image.info().unwrap(); // Let it panic because the PS4 assume it is available.

        // Unprotect the memory.
        let mut mem = match module.memory().unprotect() {
            Ok(v) => v,
            Err(e) => return Err(RelocateError::UnprotectFailed(path.to_owned(), e)),
        };

        // Apply relocations.
        let base = module.memory().base();

        self.relocate_rela(path, info, mem.as_mut(), base)?;

        // TODO: Implement the remaining relocate_one_object.
        Ok(())
    }

    /// See `reloc_non_plt` on the PS4 kernel for a reference.
    fn relocate_rela(
        &self,
        path: &VPath,
        info: &FileInfo,
        mem: &mut [u8],
        base: usize,
    ) -> Result<(), RelocateError> {
        let addr = mem.as_ptr() as usize;

        for reloc in info.relocs() {
            // Resolve value.
            let offset = base + reloc.offset();
            let addend: isize = reloc.addend().try_into().unwrap();
            let target = &mut mem[offset..(offset + 8)];
            let value = match reloc.ty() {
                Relocation::R_X86_64_NONE => break,
                Relocation::R_X86_64_64 => {
                    // TODO: Resolve symbol.
                    continue;
                }
                Relocation::R_X86_64_RELATIVE => {
                    // TODO: Apply checks from reloc_non_plt.
                    addr + base.wrapping_add_signed(addend)
                }
                Relocation::R_X86_64_DTPMOD64 => {
                    // TODO: Resolve symbol.
                    continue;
                }
                v => return Err(RelocateError::UnsupportedRela(path.to_owned(), v)),
            };

            // Write the value.
            unsafe { std::ptr::write_unaligned(target.as_mut_ptr() as _, value) };
        }

        Ok(())
    }
}

/// Represents the error for [`RuntimeLinker`] initialization.
#[derive(Debug, Error)]
pub enum RuntimeLinkerError {
    #[error("{0} does not exists")]
    ExeNotFound(VPathBuf),

    #[error("cannot open {0}")]
    OpenExeFailed(VPathBuf, #[source] std::io::Error),

    #[error("cannot open {0}")]
    OpenElfFailed(VPathBuf, #[source] elf::OpenError),

    #[error("{0} is not a valid executable")]
    InvalidExe(VPathBuf),

    #[error("cannot map {0}")]
    MapExeFailed(VPathBuf, #[source] MapError),
}

/// Represents an error for (S)ELF mapping.
#[derive(Debug, Error)]
pub enum MapError {
    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] MmapError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] ReadProgramError),

    #[error("cannot protect the memory")]
    ProtectMemoryFailed(#[source] MprotectError),
}

/// Represents an error for (S)ELF loading.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("the specified file does not exists")]
    FileNotFound,

    #[error("cannot open file")]
    OpenFileFailed(#[source] std::io::Error),

    #[error("cannot open (S)ELF")]
    OpenElfFailed(#[source] elf::OpenError),

    #[error("the specified file is not valid module")]
    InvalidElf,

    #[error("cannot map file")]
    MapFailed(#[source] MapError),
}

/// Represents an error for modules relocation.
#[derive(Debug, Error)]
pub enum RelocateError {
    #[error("cannot unprotect the memory of {0}")]
    UnprotectFailed(VPathBuf, #[source] MprotectError),

    #[error("relocation type {1} on {0} is not supported")]
    UnsupportedRela(VPathBuf, u32),
}

impl Errno for RelocateError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::UnprotectFailed(_, e) => e.errno(),
            Self::UnsupportedRela(_, _) => ENOEXEC,
        }
    }
}
