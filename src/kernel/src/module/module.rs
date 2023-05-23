use super::{LoadError, Memory};
use crate::memory::{MemoryManager, MprotectError};
use byteorder::{ByteOrder, NativeEndian};
use elf::dynamic::{DynamicLinking, ModuleFlags, RelocationInfo, SymbolInfo};
use elf::Elf;
use std::error::Error;
use std::fs::File;
use thiserror::Error;

/// Represents a loaded SELF in an unmodified state (no code lifting, etc.). That is, the same
/// representation as on PS4.
pub struct Module<'a> {
    id: u64,
    image: Elf<File>,
    memory: Memory<'a>,
}

impl<'a> Module<'a> {
    pub(super) fn load(
        id: u64,
        mut image: Elf<File>,
        mm: &'a MemoryManager,
    ) -> Result<Self, LoadError> {
        // Map SELF to the memory.
        let mut memory = Memory::new(&image, mm)?;

        memory.load(|prog, buf| {
            if let Err(e) = image.read_program(prog, buf) {
                Err(LoadError::ReadProgramFailed(prog, e))
            } else {
                Ok(())
            }
        })?;

        if let Err(e) = memory.protect() {
            return Err(LoadError::ProtectionMemoryFailed(e));
        }

        Ok(Self { id, image, memory })
    }

    pub fn image(&self) -> &Elf<File> {
        &self.image
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// # Safety
    /// No other threads may not read/write/execute the module memory.
    pub unsafe fn apply_relocs<R, E>(&self, mut resolver: R) -> Result<(), RelocError<E>>
    where
        R: FnMut(u32, &str) -> Result<usize, E>,
        E: Error,
    {
        // Do nothing if the module is not dynamic linking.
        let dynamic = match self.image.dynamic_linking() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Unprotect the memory.
        let mut mem = match self.memory.unprotect() {
            Ok(v) => v,
            Err(e) => return Err(RelocError::UnprotectMemoryFailed(e)),
        };

        // Apply relocation.
        let base = mem.addr();

        for (i, reloc) in dynamic.relocation_entries().enumerate() {
            let target = &mut mem[reloc.offset()..];
            let addend = reloc.addend();

            match reloc.ty() {
                RelocationInfo::R_X86_64_64 => {
                    // Get target symbol.
                    let symbol = match dynamic.symbols().get(reloc.symbol()) {
                        Some(v) => v,
                        None => return Err(RelocError::InvalidSymbolIndex(i)),
                    };

                    // Check binding type.
                    let value = match symbol.binding() {
                        SymbolInfo::STB_GLOBAL | SymbolInfo::STB_WEAK => {
                            match self.resolve_external_symbol(symbol, dynamic, &mut resolver) {
                                Ok(v) => v,
                                Err(e) => {
                                    return Err(RelocError::ResolveSymbolFailed(
                                        symbol.name().to_owned(),
                                        e,
                                    ));
                                }
                            }
                        }
                        v => {
                            return Err(RelocError::UnknownSymbolBinding(
                                symbol.name().to_owned(),
                                v,
                            ));
                        }
                    };

                    NativeEndian::write_u64(target, (value + addend) as u64);
                }
                RelocationInfo::R_X86_64_RELATIVE => {
                    NativeEndian::write_u64(target, (base + addend) as u64);
                }
                RelocationInfo::R_X86_64_DTPMOD64 => {
                    // Uplift add to the value instead of replacing it. According to
                    // https://chao-tic.github.io/blog/2018/12/25/tls it should be replaced with the
                    // module ID. Let's follow the standard way until something is broken.
                    NativeEndian::write_u64(target, self.id);
                }
                v => return Err(RelocError::UnknownRelocationType(i, v)),
            }
        }

        // Apply Procedure Linkage Table relocation.
        for (i, reloc) in dynamic.plt_relocation().enumerate() {
            if reloc.ty() != RelocationInfo::R_X86_64_JUMP_SLOT {
                return Err(RelocError::UnknownPltRelocType(i, reloc.ty()));
            }

            // Get target symbol.
            let symbol = match dynamic.symbols().get(reloc.symbol()) {
                Some(v) => v,
                None => return Err(RelocError::InvalidPltSymIndex(i)),
            };

            // Check binding type.
            let value = match symbol.binding() {
                SymbolInfo::STB_GLOBAL => {
                    match self.resolve_external_symbol(symbol, dynamic, &mut resolver) {
                        Ok(v) => v,
                        Err(e) => {
                            return Err(RelocError::ResolvePltSymFailed(
                                symbol.name().to_owned(),
                                e,
                            ));
                        }
                    }
                }
                v => {
                    return Err(RelocError::UnknownPltSymBinding(
                        symbol.name().to_owned(),
                        v,
                    ));
                }
            };

            // Write the target.
            let target = &mut mem[reloc.offset()..];

            NativeEndian::write_u64(target, value as _);
        }

        Ok(())
    }

    fn resolve_external_symbol<R, E>(
        &self,
        sym: &SymbolInfo,
        data: &DynamicLinking,
        resolver: &mut R,
    ) -> Result<usize, ExternalSymbolError<E>>
    where
        R: FnMut(u32, &str) -> Result<usize, E>,
        E: Error,
    {
        // Decode symbol name.
        let (name, library, module) = match sym.decode_name() {
            Some(v) => v,
            None => return Err(ExternalSymbolError::InvalidName),
        };

        // Get module where the symbol belong.
        let module = if module == 0 {
            data.module_info()
        } else {
            match data.dependencies().get(&module) {
                Some(v) => v,
                None => return Err(ExternalSymbolError::InvalidModule(module)),
            }
        };

        // Get library where the symbol belong.
        let library = match data.libraries().get(&library) {
            Some(v) => v,
            None => return Err(ExternalSymbolError::InvalidLibrary(library)),
        };

        // Get name hash.
        let name = format!("{}#{}#{}", name, library.name(), module.name());
        let hash = Self::hash_symbol(&name);

        // Resolve from self first if it is symbolic.
        if let Some(flags) = data.flags() {
            if flags.contains(ModuleFlags::DF_SYMBOLIC) {
                if let Some(sym) = data.lookup_symbol(hash, &name) {
                    return Ok(self.memory.addr() + sym.value());
                }
            }
        }

        // Invoke resolver.
        match resolver(hash, &name) {
            Ok(v) => Ok(v),
            Err(e) => Err(ExternalSymbolError::ResolveFailed(name, hash, e)),
        }
    }

    fn hash_symbol(name: &str) -> u32 {
        let mut h = 0u32;
        let mut g;

        for b in name.bytes() {
            h = (h << 4) + (b as u32);
            g = h & 0xf0000000;
            if g != 0 {
                h ^= g >> 24;
            }
            h &= !g;
        }

        h
    }
}

/// Represents the errors for [`Module::apply_relocs()`].
#[derive(Debug, Error)]
pub enum RelocError<R: Error> {
    #[error("cannot unprotect the memory")]
    UnprotectMemoryFailed(#[source] MprotectError),

    #[error("unknown relocation type {1:#010x} on entry {0}")]
    UnknownRelocationType(usize, u32),

    #[error("invalid symbol index on entry {0}")]
    InvalidSymbolIndex(usize),

    #[error("unknown symbol binding type {1} on symbol {0}")]
    UnknownSymbolBinding(String, u8),

    #[error("cannot resolve symbol {0}")]
    ResolveSymbolFailed(String, #[source] ExternalSymbolError<R>),

    #[error("unknown PLT relocation type {1:#010x} on entry {0}")]
    UnknownPltRelocType(usize, u32),

    #[error("invalid symbol index on PLT entry {0}")]
    InvalidPltSymIndex(usize),

    #[error("unknown symbol binding type {1} on PLT symbol {0}")]
    UnknownPltSymBinding(String, u8),

    #[error("cannot resolve PLT symbol {0}")]
    ResolvePltSymFailed(String, #[source] ExternalSymbolError<R>),
}

/// Represents the errors for external symbol.
#[derive(Debug, Error)]
pub enum ExternalSymbolError<R: Error> {
    #[error("invalid name")]
    InvalidName,

    #[error("module #{0} does not exists")]
    InvalidModule(u16),

    #[error("library #{0} does not exists")]
    InvalidLibrary(u16),

    #[error("cannot resolve {0} ({1:#010x})")]
    ResolveFailed(String, u32, #[source] R),
}
