use super::{MapError, Memory};
use crate::memory::MemoryManager;
use bitflags::bitflags;
use elf::{DynamicFlags, DynamicTag, Elf, FileInfo, LibraryFlags, LibraryInfo, ModuleInfo};
use std::fmt::{Display, Formatter};
use std::fs::File;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.h#L147.
pub struct Module<'a> {
    id: u32,
    entry: Option<usize>,
    tls_index: u32,
    proc_param: Option<(usize, usize)>,
    flags: ModuleFlags,
    needed: Vec<NeededModule>,
    modules: Vec<ModuleInfo>,
    libraries: Vec<LibraryInfo>,
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

        // Initialize PLT relocation.
        if let Some(i) = image.info() {
            Self::init_plt(&mut memory, base, i);
        }

        // Apply memory protection.
        if let Err(e) = memory.protect() {
            return Err(MapError::ProtectMemoryFailed(e));
        }

        // Parse dynamic info.
        let mut module = Self {
            id,
            entry: image.entry_addr().map(|v| base + v),
            tls_index,
            proc_param: image.proc_param().map(|i| {
                let p = image.programs().get(i).unwrap();
                (base + p.addr(), p.file_size().try_into().unwrap())
            }),
            flags: ModuleFlags::empty(),
            needed: Vec::new(),
            modules: Vec::new(),
            libraries: Vec::new(),
            image,
            memory,
        };

        module.digest_dynamic()?;

        Ok(module)
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

    pub fn flags(&self) -> ModuleFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut ModuleFlags {
        &mut self.flags
    }

    pub fn needed(&self) -> &[NeededModule] {
        self.needed.as_ref()
    }

    pub fn image(&self) -> &Elf<File> {
        &self.image
    }

    pub fn memory(&self) -> &Memory<'a> {
        &self.memory
    }

    /// See `dynlib_initialize_pltgot_each` on the PS4 for a reference.
    fn init_plt(mem: &mut Memory, base: usize, info: &FileInfo) {
        let mem = mem.as_mut();

        for (i, reloc) in info.plt_relocs().enumerate() {
            // TODO: Apply the same checks from dynlib_initialize_pltgot_each.
            let offset = base + reloc.offset();
            let dst = &mut mem[offset..(offset + 8)];

            // SAFETY: This is safe because dst is forced to be valid by the above statement.
            let i: u64 = i.try_into().unwrap();

            // Not sure why Sony initialize each PLT relocation to 0xeffffffe????????.
            unsafe { std::ptr::write_unaligned(dst.as_mut_ptr() as _, i | 0xeffffffe00000000) };
        }
    }

    /// See `digest_dynamic` on the PS4 for a reference.
    fn digest_dynamic(&mut self) -> Result<(), MapError> {
        let info = match self.image.info() {
            Some(v) => v,
            None => return Ok(()), // Do nothing if not a dynamic module.
        };

        // TODO: Implement the remaining tags.
        for (i, (tag, value)) in info.dynamic().enumerate() {
            match tag {
                DynamicTag::DT_NULL => break,
                DynamicTag::DT_NEEDED => {
                    let name = u64::from_le_bytes(value);

                    match info.read_str(name.try_into().unwrap()) {
                        Ok(v) => self.needed.push(NeededModule { name: v.to_owned() }),
                        Err(e) => return Err(MapError::ReadNeededFailed(i, e)),
                    }
                }
                DynamicTag::DT_FLAGS => {
                    let flags = DynamicFlags::from_bits_retain(u64::from_le_bytes(value));

                    if flags.contains(DynamicFlags::DF_SYMBOLIC) {
                        return Err(MapError::ObsoleteFlags(DynamicFlags::DF_SYMBOLIC));
                    } else if flags.contains(DynamicFlags::DF_BIND_NOW) {
                        return Err(MapError::ObsoleteFlags(DynamicFlags::DF_BIND_NOW));
                    } else if flags.contains(DynamicFlags::DF_TEXTREL) {
                        self.flags |= ModuleFlags::TEXT_REL;
                    }
                }
                DynamicTag::DT_SCE_MODULE_INFO | DynamicTag::DT_SCE_NEEDED_MODULE => {
                    match info.read_module(value) {
                        Ok(v) => self.modules.push(v),
                        Err(e) => return Err(MapError::ReadModuleInfoFailed(i, e)),
                    }
                }
                DynamicTag::DT_SCE_EXPORT_LIB | DynamicTag::DT_SCE_IMPORT_LIB => {
                    let mut info = match info.read_library(value) {
                        Ok(v) => v,
                        Err(e) => return Err(MapError::ReadLibraryInfoFailed(i, e)),
                    };

                    if tag == DynamicTag::DT_SCE_EXPORT_LIB {
                        *info.flags_mut() |= LibraryFlags::EXPORT;
                    }

                    self.libraries.push(info);
                }
                _ => continue,
            }
        }

        Ok(())
    }
}

bitflags! {
    /// Flags for [`Module`].
    #[derive(Clone, Copy, PartialEq)]
    pub struct ModuleFlags: u16 {
        const MAIN_PROG = 0x0001;
        const TEXT_REL = 0x0002;
        const INIT_SCANNED = 0x0010;
    }
}

impl Display for ModuleFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// An implementation of `Needed_Entry`.
pub struct NeededModule {
    name: String,
}

impl NeededModule {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
