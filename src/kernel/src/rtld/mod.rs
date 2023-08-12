pub use mem::*;
pub use module::*;

use crate::errno::{Errno, EINVAL, ENOEXEC};
use crate::fs::{Fs, VPath, VPathBuf};
use crate::memory::{MmapError, MprotectError, Protections};
use bitflags::bitflags;
use elf::{DynamicFlags, Elf, FileInfo, FileType, ReadProgramError, Relocation, Symbol};
use std::fs::File;
use std::num::NonZeroI32;
use std::ops::Deref;
use std::ptr::write_unaligned;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use thiserror::Error;

mod mem;
mod module;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.c.
#[derive(Debug)]
pub struct RuntimeLinker<'a> {
    fs: &'a Fs,
    list: RwLock<Vec<Arc<Module>>>,      // obj_list + obj_tail
    app: Arc<Module>,                    // obj_main
    kernel: RwLock<Option<Arc<Module>>>, // obj_kernel
    next_id: Mutex<u32>,                 // idtable on proc
    tls_max: Mutex<u32>,                 // tls_max_index
    flags: LinkerFlags,
}

impl<'a> RuntimeLinker<'a> {
    pub fn new(fs: &'a Fs) -> Result<Self, RuntimeLinkerError> {
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
            Ok(v) => match Elf::open(file.vpath(), v) {
                Ok(v) => v,
                Err(e) => return Err(RuntimeLinkerError::OpenElfFailed(file.into_vpath(), e)),
            },
            Err(e) => return Err(RuntimeLinkerError::OpenExeFailed(file.into_vpath(), e)),
        };

        // Check image type.
        match elf.ty() {
            FileType::ET_EXEC | FileType::ET_SCE_EXEC | FileType::ET_SCE_REPLAY_EXEC => {
                if elf.info().is_none() {
                    todo!("a statically linked eboot.bin is not supported yet.");
                }
            }
            FileType::ET_SCE_DYNEXEC if elf.dynamic().is_some() => {}
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
        let app = match Module::map(elf, base, 0, 1) {
            Ok(v) => Arc::new(v),
            Err(e) => return Err(RuntimeLinkerError::MapExeFailed(file.into_vpath(), e)),
        };

        *app.flags_mut() |= ModuleFlags::MAIN_PROG;

        // Check if application need certain modules.
        let mut flags = LinkerFlags::empty();

        for m in app.modules() {
            match m.name() {
                "libSceDbgUndefinedBehaviorSanitizer" => flags |= LinkerFlags::UNK1,
                "libSceDbgAddressSanitizer" => flags |= LinkerFlags::UNK2,
                _ => continue,
            }
        }

        // TODO: Apply logic from dmem_handle_process_exec_begin.
        // TODO: Apply logic from procexec_handler.
        // TODO: Apply logic from umtx_exec_hook.
        // TODO: Apply logic from aio_proc_rundown_exec.
        // TODO: Apply logic from gs_is_event_handler_process_exec.
        Ok(Self {
            fs,
            list: RwLock::new(Vec::from([app.clone()])),
            app,
            kernel: RwLock::default(),
            next_id: Mutex::new(1),
            tls_max: Mutex::new(1),
            flags,
        })
    }

    pub fn list(&self) -> RwLockReadGuard<'_, Vec<Arc<Module>>> {
        self.list.read().unwrap()
    }

    pub fn app(&self) -> &Arc<Module> {
        &self.app
    }

    pub fn kernel(&self) -> Option<Arc<Module>> {
        self.kernel.read().unwrap().clone()
    }

    pub fn set_kernel(&self, md: Arc<Module>) {
        *self.kernel.write().unwrap() = Some(md);
    }

    /// This method **ALWAYS** load the specified module without checking if the same module is
    /// already loaded.
    pub fn load(&mut self, path: &VPath) -> Result<Arc<Module>, LoadError> {
        // Get file.
        let file = match self.fs.get_file(path) {
            Some(v) => v,
            None => return Err(LoadError::FileNotFound),
        };

        // Open file.
        let elf = match File::open(file.path()) {
            Ok(v) => match Elf::open(file.into_vpath(), v) {
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
        // Search for TLS free slot.
        let mut list = self.list.write().unwrap();
        let mut tls_max = self.tls_max.lock().unwrap();
        let tls = elf.tls().map(|i| &elf.programs()[i]);
        let tls = if tls.map(|p| p.memory_size()).unwrap_or(0) == 0 {
            0
        } else {
            let mut r = 1;

            loop {
                // Check if the current value has been used.
                if !list.iter().any(|m| m.tls_index() == r) {
                    break;
                }

                // Someone already use the current value, increase the value and try again.
                r += 1;

                if r > *tls_max {
                    *tls_max = r;
                    break;
                }
            }

            r
        };

        // Map file.
        let mut next_id = self.next_id.lock().unwrap();
        let module = match Module::map(elf, 0, *next_id, tls) {
            Ok(v) => Arc::new(v),
            Err(e) => return Err(LoadError::MapFailed(e)),
        };

        if module.flags().contains(ModuleFlags::TEXT_REL) {
            return Err(LoadError::ImpureText);
        }

        // TODO: Check the call to sceSblAuthMgrIsLoadable in the self_load_shared_object on the PS4
        // to see how it is return the value.
        let name = path.file_name().unwrap();

        if name != "libc.sprx" && name != "libSceFios2.sprx" {
            *module.flags_mut() |= ModuleFlags::UNK1;
        }

        // Load to loaded list.
        list.push(module.clone());
        *next_id += 1;

        Ok(module)
    }

    /// # Safety
    /// No other threads may access the memory of all loaded modules.
    pub unsafe fn relocate(&self) -> Result<(), RelocateError> {
        // TODO: Check what the PS4 actually doing.
        for m in self.list.read().unwrap().deref() {
            self.relocate_single(m)?;
        }

        Ok(())
    }

    /// See `relocate_one_object` on the PS4 kernel for a reference.
    ///
    /// # Safety
    /// No other thread may access the module memory.
    unsafe fn relocate_single(&self, md: &Arc<Module>) -> Result<(), RelocateError> {
        // Unprotect the memory.
        let mut mem = match md.memory().unprotect() {
            Ok(v) => v,
            Err(e) => return Err(RelocateError::UnprotectFailed(md.path().to_owned(), e)),
        };

        // Apply relocations.
        let mut relocated = md.relocated();

        self.relocate_rela(md, mem.as_mut(), &mut relocated)?;

        if !md.flags().contains(ModuleFlags::UNK4) {
            self.relocate_plt(md, mem.as_mut(), &mut relocated)?;
        }

        Ok(())
    }

    /// See `reloc_non_plt` on the PS4 kernel for a reference.
    fn relocate_rela(
        &self,
        md: &Arc<Module>,
        mem: &mut [u8],
        relocated: &mut [bool],
    ) -> Result<(), RelocateError> {
        let info = md.file_info().unwrap(); // Let it panic because the PS4 assume it is available.
        let addr = mem.as_ptr() as usize;
        let base = md.memory().base();

        for (i, reloc) in info.relocs().enumerate() {
            // Check if the entry already relocated.
            if relocated[i] {
                continue;
            }

            // Resolve value.
            let offset = base + reloc.offset();
            let target = &mut mem[offset..(offset + 8)];
            let addend = reloc.addend();
            let value = match reloc.ty() {
                Relocation::R_X86_64_NONE => break,
                Relocation::R_X86_64_64 => {
                    // TODO: Resolve symbol.
                    continue;
                }
                Relocation::R_X86_64_RELATIVE => {
                    // TODO: Apply checks from reloc_non_plt.
                    (addr + base).wrapping_add_signed(addend)
                }
                Relocation::R_X86_64_DTPMOD64 => {
                    // TODO: Resolve symbol.
                    continue;
                }
                v => return Err(RelocateError::UnsupportedRela(md.path().to_owned(), v)),
            };

            // Write the value.
            unsafe { write_unaligned(target.as_mut_ptr() as *mut usize, value) };

            relocated[i] = true;
        }

        Ok(())
    }

    /// See `reloc_jmplots` on the PS4 for a reference.
    fn relocate_plt(
        &self,
        md: &Arc<Module>,
        mem: &mut [u8],
        relocated: &mut [bool],
    ) -> Result<(), RelocateError> {
        // Do nothing if not a dynamic module.
        let info = match md.file_info() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Apply relocations.
        let base = md.memory().base();

        for (i, reloc) in info.plt_relocs().enumerate() {
            // Check if the entry already relocated.
            let index = info.reloc_count() + i;

            if relocated[index] {
                continue;
            }

            // Check relocation type.
            if reloc.ty() != Relocation::R_X86_64_JUMP_SLOT {
                return Err(RelocateError::UnsupportedPlt(
                    md.path().to_owned(),
                    reloc.ty(),
                ));
            }

            // Resolve symbol.
            let sym = match self.resolve_symbol(md, reloc.symbol(), info) {
                Some((m, s)) => {
                    m.memory().addr() + m.memory().base() + m.symbol(s).unwrap().value()
                }
                None => continue,
            };

            // Write the value.
            let offset = base + reloc.offset();
            let target = &mut mem[offset..(offset + 8)];
            let value = sym.wrapping_add_signed(reloc.addend());

            unsafe { write_unaligned(target.as_mut_ptr() as *mut usize, value) };

            relocated[index] = true;
        }

        Ok(())
    }

    fn resolve_symbol(
        &self,
        md: &Arc<Module>,
        index: usize,
        info: &FileInfo,
    ) -> Option<(Arc<Module>, usize)> {
        // Check if symbol index is valid.
        let sym = md.symbols().get(index)?;

        if index >= info.nchains() {
            return None;
        }

        if self.app.sdk_ver() >= 0x5000000 || self.flags.contains(LinkerFlags::UNK2) {
            // Get library and module.
            let (li, mi) = match sym.decode_name() {
                Some(v) => (
                    md.libraries().iter().find(|&l| l.id() == v.1),
                    md.modules().iter().find(|&m| m.id() == v.2),
                ),
                None => (None, None),
            };
        } else {
            todo!("resolve symbol with SDK version < 0x5000000");
        }

        // Return this symbol if the binding is local. The reason we don't check this in the
        // first place is because we want to maintain the same behavior as the PS4.
        if sym.binding() == Symbol::STB_LOCAL {
            return Some((md.clone(), index));
        }

        None
    }
}

bitflags! {
    /// Flags for [`RuntimeLinker`].
    #[derive(Debug)]
    pub struct LinkerFlags: u8 {
        const UNK1 = 0x01; // TODO: Rename this.
        const UNK2 = 0x02; // TODO: Rename this.
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
    #[error("the image has multiple executable programs")]
    MultipleExecProgram,

    #[error("the image has multiple data programs")]
    MultipleDataProgram,

    #[error("the image has multiple PT_SCE_RELRO")]
    MultipleRelroProgram,

    #[error("ELF program {0} has invalid alignment")]
    InvalidProgramAlignment(usize),

    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] MmapError),

    #[error("cannot protect {1:#018x} bytes starting at {0:p} with {2}")]
    ProtectMemoryFailed(*const u8, usize, Protections, #[source] MprotectError),

    #[error("cannot unprotect segment {0}")]
    UnprotectSegmentFailed(usize, #[source] UnprotectSegmentError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] ReadProgramError),

    #[error("cannot unprotect the memory")]
    UnprotectMemoryFailed(#[source] UnprotectError),

    #[error("cannot read symbol entry {0}")]
    ReadSymbolFailed(usize, #[source] elf::ReadSymbolError),

    #[error("cannot read DT_NEEDED from dynamic entry {0}")]
    ReadNeededFailed(usize, #[source] elf::StringTableError),

    #[error("{0} is obsolete")]
    ObsoleteFlags(DynamicFlags),

    #[error("cannot read module info from dynamic entry {0}")]
    ReadModuleInfoFailed(usize, #[source] elf::ReadModuleError),

    #[error("cannot read libraru info from dynamic entry {0}")]
    ReadLibraryInfoFailed(usize, #[source] elf::ReadLibraryError),
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

    #[error("the specified file has impure text")]
    ImpureText,
}

/// Represents an error for modules relocation.
#[derive(Debug, Error)]
pub enum RelocateError {
    #[error("cannot unprotect the memory of {0}")]
    UnprotectFailed(VPathBuf, #[source] UnprotectError),

    #[error("relocation type {1} on {0} is not supported")]
    UnsupportedRela(VPathBuf, u32),

    #[error("PLT relocation type {1} on {0} is not supported")]
    UnsupportedPlt(VPathBuf, u32),
}

impl Errno for RelocateError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::UnprotectFailed(_, e) => match e {
                UnprotectError::MprotectFailed(_, _, _, e) => e.errno(),
            },
            Self::UnsupportedRela(_, _) => ENOEXEC,
            Self::UnsupportedPlt(_, _) => EINVAL,
        }
    }
}
