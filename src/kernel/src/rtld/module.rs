use super::{MapError, Memory};
use crate::fs::{VPath, VPathBuf};
use crate::log::{print, LogEntry};
use crate::memory::MemoryManager;
use bitflags::bitflags;
use elf::{
    DynamicFlags, DynamicTag, Elf, FileInfo, FileType, LibraryFlags, LibraryInfo, ModuleInfo,
    Program,
};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Write;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.h#L147.
pub struct Module<'a> {
    id: u32,
    init: Option<usize>,
    entry: Option<usize>,
    fini: Option<usize>,
    tls_index: u32,
    tls_init: Option<usize>,
    proc_param: Option<(usize, usize)>,
    flags: ModuleFlags,
    needed: Vec<NeededModule>,
    modules: Vec<ModuleInfo>,
    libraries: Vec<LibraryInfo>,
    memory: Memory<'a>,
    file_info: Option<FileInfo>,
    path: VPathBuf,
    is_self: bool,
    file_type: FileType,
    programs: Vec<Program>,
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

        // Extract image info.
        let entry = image.entry_addr().map(|v| base + v);

        // TODO: Check if PT_TLS with zero value is valid or not. If not, set this to None instead.
        let tls_init = image
            .tls()
            .map(|i| image.program(i).unwrap())
            .map(|p| base + p.addr());
        let proc_param = image
            .proc_param()
            .map(|i| image.program(i).unwrap())
            .map(|p| (base + p.addr(), p.file_size().try_into().unwrap()));
        let is_self = image.self_segments().is_some();
        let file_type = image.ty();
        let (path, programs, file_info) = image.into();

        // Parse dynamic info.
        let mut module = Self {
            id,
            init: None,
            entry,
            fini: None,
            tls_index,
            tls_init,
            proc_param,
            flags: ModuleFlags::UNK2,
            needed: Vec::new(),
            modules: Vec::new(),
            libraries: Vec::new(),
            memory,
            file_info: None,
            path: path.try_into().unwrap(),
            is_self,
            file_type,
            programs,
        };

        if let Some(info) = file_info {
            module.digest_dynamic(base, &info)?;
            module.file_info = Some(info);
        }

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

    pub fn tls_init(&self) -> Option<usize> {
        self.tls_init
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

    pub fn memory(&self) -> &Memory<'a> {
        &self.memory
    }

    /// Only available if the module is a dynamic module.
    pub fn file_info(&self) -> Option<&FileInfo> {
        self.file_info.as_ref()
    }

    pub fn path(&self) -> &VPath {
        &self.path
    }

    pub fn print(&self, mut entry: LogEntry) {
        // Image info.
        if self.is_self {
            writeln!(entry, "Image format  : SELF").unwrap();
        } else {
            writeln!(entry, "Image format  : ELF").unwrap();
        }

        writeln!(entry, "Image type    : {}", self.file_type).unwrap();

        for (i, p) in self.programs.iter().enumerate() {
            writeln!(
                entry,
                "Program {:<6}: {:#018x}:{:#018x} => {:#018x}:{:#018x} {}",
                i,
                p.offset(),
                p.offset() + p.file_size(),
                p.addr(),
                p.addr() + p.memory_size(),
                p.ty()
            )
            .unwrap();
        }

        for n in &self.needed {
            writeln!(entry, "Needed        : {}", n.name).unwrap();
        }

        // Runtime info.
        if !self.flags.is_empty() {
            writeln!(entry, "Module flags  : {}", self.flags).unwrap();
        }

        writeln!(entry, "TLS index     : {}", self.tls_index).unwrap();

        // Memory info.
        let mem = self.memory();

        writeln!(
            entry,
            "Memory address: {:#018x}:{:#018x}",
            mem.addr(),
            mem.addr() + mem.len()
        )
        .unwrap();

        if let Some(v) = self.init {
            writeln!(entry, "Initialization: {:#018x}", mem.addr() + v).unwrap();
        }

        if let Some(v) = self.entry {
            writeln!(entry, "Entry address : {:#018x}", mem.addr() + v).unwrap();
        }

        if let Some(v) = self.fini {
            writeln!(entry, "Finalization  : {:#018x}", mem.addr() + v).unwrap();
        }

        if let Some((off, size)) = &self.proc_param {
            let addr = mem.addr() + off;

            writeln!(
                entry,
                "Process param : {:#018x}:{:#018x}",
                addr,
                addr + size
            )
            .unwrap();
        }

        if let Some(v) = self.tls_init {
            writeln!(entry, "TLS init      : {:#018x}", mem.addr() + v).unwrap();
        }

        for s in mem.segments().iter() {
            let p = match s.program() {
                Some(v) => v,
                None => continue,
            };

            let addr = mem.addr() + s.start();

            writeln!(
                entry,
                "Program {} is mapped to {:#018x}:{:#018x} with {}.",
                p,
                addr,
                addr + s.len(),
                self.programs[p].flags(),
            )
            .unwrap();
        }

        print(entry);
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
    fn digest_dynamic(&mut self, base: usize, info: &FileInfo) -> Result<(), MapError> {
        // TODO: Implement the remaining tags.
        for (i, (tag, value)) in info.dynamic().enumerate() {
            match tag {
                DynamicTag::DT_NULL => break,
                DynamicTag::DT_NEEDED => self.digest_needed(info, i, value)?,
                DynamicTag::DT_PLTRELSZ
                | DynamicTag::DT_HASH
                | DynamicTag::DT_STRTAB
                | DynamicTag::DT_SYMTAB
                | DynamicTag::DT_RELA
                | DynamicTag::DT_RELASZ
                | DynamicTag::DT_RELAENT
                | DynamicTag::DT_STRSZ
                | DynamicTag::DT_SYMENT
                | DynamicTag::DT_PLTREL
                | DynamicTag::DT_DEBUG
                | DynamicTag::DT_JMPREL
                | DynamicTag::DT_INIT_ARRAY
                | DynamicTag::DT_FINI_ARRAY
                | DynamicTag::DT_INIT_ARRAYSZ
                | DynamicTag::DT_FINI_ARRAYSZ
                | DynamicTag::DT_PREINIT_ARRAY
                | DynamicTag::DT_PREINIT_ARRAYSZ => {
                    // These tags are ignored by the PS4.
                }
                DynamicTag::DT_INIT => self.digest_init(base, value)?,
                DynamicTag::DT_FINI => self.digest_fini(base, value)?,
                DynamicTag::DT_TEXTREL => self.flags |= ModuleFlags::TEXT_REL,
                DynamicTag::DT_FLAGS => self.digest_flags(value)?,
                DynamicTag::DT_SCE_MODULE_INFO | DynamicTag::DT_SCE_NEEDED_MODULE => {
                    self.digest_module_info(info, i, value)?;
                }
                DynamicTag::DT_SCE_EXPORT_LIB | DynamicTag::DT_SCE_IMPORT_LIB => {
                    self.digest_library_info(info, i, value, tag == DynamicTag::DT_SCE_EXPORT_LIB)?;
                }
                _ => continue,
            }
        }

        Ok(())
    }

    fn digest_needed(&mut self, info: &FileInfo, i: usize, value: [u8; 8]) -> Result<(), MapError> {
        let name = u64::from_le_bytes(value);
        let name = match info.read_str(name.try_into().unwrap()) {
            Ok(v) => v,
            Err(e) => return Err(MapError::ReadNeededFailed(i, e)),
        };

        self.needed.push(NeededModule {
            name: name.to_owned(),
        });

        Ok(())
    }

    fn digest_init(&mut self, base: usize, value: [u8; 8]) -> Result<(), MapError> {
        // TODO: Apply checks from digest_dynamic on the PS4.
        let addr: usize = u64::from_le_bytes(value).try_into().unwrap();

        if addr != 0 {
            self.init = Some(base + addr);
        }

        Ok(())
    }

    fn digest_fini(&mut self, base: usize, value: [u8; 8]) -> Result<(), MapError> {
        // TODO: Apply checks from digest_dynamic on the PS4.
        let addr: usize = u64::from_le_bytes(value).try_into().unwrap();

        if addr != 0 {
            self.fini = Some(base + addr);
        }

        Ok(())
    }

    fn digest_flags(&mut self, value: [u8; 8]) -> Result<(), MapError> {
        let flags = DynamicFlags::from_bits_retain(u64::from_le_bytes(value));

        if flags.contains(DynamicFlags::DF_SYMBOLIC) {
            return Err(MapError::ObsoleteFlags(DynamicFlags::DF_SYMBOLIC));
        } else if flags.contains(DynamicFlags::DF_BIND_NOW) {
            return Err(MapError::ObsoleteFlags(DynamicFlags::DF_BIND_NOW));
        } else if flags.contains(DynamicFlags::DF_TEXTREL) {
            self.flags |= ModuleFlags::TEXT_REL;
        }

        Ok(())
    }

    fn digest_module_info(
        &mut self,
        info: &FileInfo,
        i: usize,
        value: [u8; 8],
    ) -> Result<(), MapError> {
        let module = match info.read_module(value) {
            Ok(v) => v,
            Err(e) => return Err(MapError::ReadModuleInfoFailed(i, e)),
        };

        self.modules.push(module);
        Ok(())
    }

    fn digest_library_info(
        &mut self,
        info: &FileInfo,
        i: usize,
        value: [u8; 8],
        export: bool,
    ) -> Result<(), MapError> {
        let mut info = match info.read_library(value) {
            Ok(v) => v,
            Err(e) => return Err(MapError::ReadLibraryInfoFailed(i, e)),
        };

        if export {
            *info.flags_mut() |= LibraryFlags::EXPORT;
        }

        self.libraries.push(info);
        Ok(())
    }
}

bitflags! {
    /// Flags for [`Module`].
    #[derive(Clone, Copy, PartialEq)]
    pub struct ModuleFlags: u16 {
        const MAIN_PROG = 0x0001;
        const TEXT_REL = 0x0002;
        const JMPSLOTS_DONE = 0x0004;
        const TLS_DONE = 0x0008;
        const INIT_SCANNED = 0x0010;
        const ON_FINI_LIST = 0x0020;
        const DAG_INITED = 0x0040;
        const UNK1 = 0x0100; // TODO: Rename this.
        const UNK2 = 0x0200; // TODO: Rename this.
        const UNK3 = 0x0400; // TODO: Rename this.
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
