use super::{MapError, Memory};
use crate::fs::{VPath, VPathBuf};
use crate::log::{print, LogEntry};
use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use elf::{
    DynamicFlags, DynamicTag, Elf, FileInfo, FileType, LibraryFlags, LibraryInfo, ModuleInfo,
    Program, Symbol,
};
use gmtx::{GroupMutex, GroupMutexReadGuard, GroupMutexWriteGuard, MutexGroup};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.h#L147.
#[derive(Debug)]
pub struct Module {
    id: u32,
    init: Option<usize>,
    entry: Option<usize>,
    fini: Option<usize>,
    tls_index: u32,                // tlsindex
    tls_offset: GroupMutex<usize>, // tlsoffset
    tls_info: Option<ModuleTls>,   // tlsinit + tlsinitsize + tlssize + tlsalign
    eh_info: Option<ModuleEh>,
    proc_param: Option<(usize, usize)>,
    sdk_ver: u32,
    flags: GroupMutex<ModuleFlags>,
    needed: Vec<NeededModule>,
    modules: Vec<ModuleInfo>,
    libraries: Vec<LibraryInfo>,
    memory: Memory,
    relocated: GroupMutex<Vec<bool>>,
    file_info: Option<FileInfo>,
    path: VPathBuf,
    is_self: bool,
    file_type: FileType,
    programs: Vec<Program>,
    symbols: Vec<Symbol>,
}

impl Module {
    pub(super) fn map(
        mut image: Elf<File>,
        base: usize,
        id: u32,
        tls_index: u32,
        mtxg: &Arc<MutexGroup>,
    ) -> Result<Self, MapError> {
        // Map the image to the memory.
        let memory = Memory::new(&image, base, mtxg)?;

        for (i, s) in memory.segments().iter().enumerate() {
            // Get target program.
            let p = match s.program() {
                Some(v) => v,
                None => continue,
            };

            // Unprotect the segment.
            let mut s = match unsafe { memory.unprotect_segment(i) } {
                Ok(v) => v,
                Err(e) => return Err(MapError::UnprotectSegmentFailed(i, e)),
            };

            // Read ELF program.
            if let Err(e) = image.read_program(p, s.as_mut()) {
                return Err(MapError::ReadProgramFailed(p, e));
            }
        }

        // Parse EH frame headers.
        let eh_info = match image.eh().map(|i| image.program(i).unwrap()) {
            Some(p) => {
                assert_ne!(p.addr(), 0);
                assert_ne!(p.memory_size(), 0);

                let header = base + p.addr();
                let header_size = p.memory_size();
                let (frame, frame_size) = unsafe { Self::digest_eh(&memory, header, header_size) };

                Some(ModuleEh {
                    header,
                    header_size,
                    frame,
                    frame_size,
                })
            }
            None => None,
        };

        // Initialize PLT relocation.
        if let Some(i) = image.info() {
            unsafe { Self::init_plt(&memory, base, i)? };
        }

        // Extract image info.
        let entry = image.entry_addr().map(|v| base + v);

        // TODO: Check if PT_TLS with zero value is valid or not. If not, set this to None instead.
        let tls_info = image
            .tls()
            .map(|i| image.program(i).unwrap())
            .map(|p| ModuleTls {
                init: base + p.addr(),
                init_size: p.file_size().try_into().unwrap(),
                size: p.memory_size(),
                align: p.alignment(),
            });
        let proc_param = image
            .proc_param()
            .map(|i| image.program(i).unwrap())
            .map(|p| (base + p.addr(), p.file_size().try_into().unwrap()));
        let is_self = image.self_segments().is_some();
        let file_type = image.ty();
        let (path, programs, file_info) = image.into();

        // Load symbols.
        let symbols = if let Some(info) = &file_info {
            let mut r = Vec::with_capacity(info.symbol_count());

            for (i, s) in info.symbols().enumerate() {
                match s {
                    Ok(s) => r.push(s),
                    Err(e) => return Err(MapError::ReadSymbolFailed(i, e)),
                }
            }

            r
        } else {
            Vec::new()
        };

        // Get SDK version.
        let sdk_ver = match &proc_param {
            Some((off, _)) => unsafe { LE::read_u32(&memory.as_bytes()[(off + 0x10)..]) },
            None => 0,
        };

        // Parse dynamic info.
        let mut module = Self {
            id,
            init: None,
            entry,
            fini: None,
            tls_index,
            tls_offset: mtxg.new_member(0),
            tls_info,
            eh_info,
            proc_param,
            sdk_ver,
            flags: mtxg.new_member(ModuleFlags::UNK2),
            needed: Vec::new(),
            modules: Vec::new(),
            libraries: Vec::new(),
            memory,
            relocated: mtxg.new_member(Vec::new()),
            file_info: None,
            path: path.try_into().unwrap(),
            is_self,
            file_type,
            programs,
            symbols,
        };

        if let Some(info) = file_info {
            module.digest_dynamic(base, &info)?;
            module.relocated = mtxg.new_member(vec![false; info.reloc_count() + info.plt_count()]);
            module.file_info = Some(info);
        }

        Ok(module)
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn init(&self) -> Option<usize> {
        self.init
    }

    pub fn entry(&self) -> Option<usize> {
        self.entry
    }

    pub fn fini(&self) -> Option<usize> {
        self.fini
    }

    pub fn tls_index(&self) -> u32 {
        self.tls_index
    }

    pub fn tls_offset(&self) -> GroupMutexReadGuard<'_, usize> {
        self.tls_offset.read()
    }

    pub fn tls_offset_mut(&self) -> GroupMutexWriteGuard<'_, usize> {
        self.tls_offset.write()
    }

    pub fn tls_info(&self) -> Option<&ModuleTls> {
        self.tls_info.as_ref()
    }

    pub fn eh_info(&self) -> Option<&ModuleEh> {
        self.eh_info.as_ref()
    }

    pub fn proc_param(&self) -> Option<&(usize, usize)> {
        self.proc_param.as_ref()
    }

    pub fn sdk_ver(&self) -> u32 {
        self.sdk_ver
    }

    pub fn flags(&self) -> GroupMutexReadGuard<'_, ModuleFlags> {
        self.flags.read()
    }

    pub fn flags_mut(&self) -> GroupMutexWriteGuard<'_, ModuleFlags> {
        self.flags.write()
    }

    pub fn modules(&self) -> &[ModuleInfo] {
        self.modules.as_ref()
    }

    pub fn libraries(&self) -> &[LibraryInfo] {
        self.libraries.as_ref()
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn relocated_mut(&self) -> GroupMutexWriteGuard<'_, Vec<bool>> {
        self.relocated.write()
    }

    /// Only available if the module is a dynamic module.
    pub fn file_info(&self) -> Option<&FileInfo> {
        self.file_info.as_ref()
    }

    pub fn path(&self) -> &VPath {
        &self.path
    }

    pub fn symbol(&self, i: usize) -> Option<&Symbol> {
        self.symbols.get(i)
    }

    pub fn symbols(&self) -> &[Symbol] {
        self.symbols.as_ref()
    }

    pub fn print(&self, mut entry: LogEntry) {
        // Lock all required fields first so the output is consistent.
        let flags = self.flags.read();

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

        writeln!(entry, "Symbol count  : {}", self.symbols.len()).unwrap();

        // Runtime info.
        if !flags.is_empty() {
            writeln!(entry, "Module flags  : {}", flags).unwrap();
        }

        writeln!(entry, "TLS index     : {}", self.tls_index).unwrap();

        // Memory info.
        let mem = &self.memory;

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

        if let Some(i) = &self.tls_info {
            let init = mem.addr() + i.init;

            writeln!(
                entry,
                "TLS init      : {:#018x}:{:#018x}",
                init,
                init + i.init_size
            )
            .unwrap();

            writeln!(entry, "TLS size      : {}", i.size).unwrap();
        }

        if let Some(i) = &self.eh_info {
            let hdr = mem.addr() + i.header;
            let frame = mem.addr() + i.frame;

            writeln!(
                entry,
                "EH header     : {:#018x}:{:#018x}",
                hdr,
                hdr + i.header_size
            )
            .unwrap();

            writeln!(
                entry,
                "EH frame      : {:#018x}:{:#018x}",
                frame,
                frame + i.frame_size
            )
            .unwrap();
        }

        if let Some((off, len)) = &self.proc_param {
            let addr = mem.addr() + off;

            writeln!(entry, "Process param : {:#018x}:{:#018x}", addr, addr + len).unwrap();
            writeln!(entry, "SDK version   : {:#010x}", self.sdk_ver).unwrap();
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

    unsafe fn digest_eh(mem: &Memory, off: usize, len: usize) -> (usize, usize) {
        // Get frame header.
        let mem = mem.as_bytes();
        let hdr = &mem[off..(off + len)];

        // Let it panic because the PS4 assume the index is valid.
        let frame: isize = LE::read_i32(&hdr[4..]).try_into().unwrap();

        assert_ne!(frame, 0);

        // Get first frame.
        let frame: usize = match hdr[1] {
            3 => todo!("EH frame header type 3"),
            27 => (off + 4).checked_add_signed(frame).unwrap(),
            _ => return (0, 0),
        };

        // Get frame size.
        let mut next = frame;
        let mut total = 0;

        loop {
            let mut size: usize = LE::read_u32(&mem[next..]).try_into().unwrap();

            match size {
                0 => {
                    total += 4;
                    break;
                }
                0xffffffff => {
                    size = LE::read_u64(&mem[(next + 4)..]).try_into().unwrap();
                    size += 12;
                }
                _ => size += 4,
            }

            // TODO: Check if total size does not out of text segment.
            next += size;
            total += size;
        }

        (frame, total)
    }

    /// See `dynlib_initialize_pltgot_each` on the PS4 for a reference.
    ///
    /// # Safety
    /// No other threads may access the memory.
    unsafe fn init_plt(mem: &Memory, base: usize, info: &FileInfo) -> Result<(), MapError> {
        // Unprotect the memory.
        let mut mem = match mem.unprotect() {
            Ok(v) => v,
            Err(e) => return Err(MapError::UnprotectMemoryFailed(e)),
        };

        // Initialize all PLT entries.
        let mem = mem.as_mut();

        for (i, reloc) in info.plt_relocs().enumerate() {
            // TODO: Apply the same checks from dynlib_initialize_pltgot_each.
            let offset = base + reloc.offset();
            let dst = &mut mem[offset..(offset + 8)];

            // SAFETY: This is safe because dst is forced to be valid by the above statement.
            let i: u64 = i.try_into().unwrap();

            // Not sure why Sony initialize each PLT relocation to 0xeffffffe????????. My guess is
            // that they use this value to catch unpatched PLT entry.
            std::ptr::write_unaligned(dst.as_mut_ptr() as _, i | 0xeffffffe00000000);
        }

        Ok(())
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
                DynamicTag::DT_TEXTREL => *self.flags.get_mut() |= ModuleFlags::TEXT_REL,
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
            *self.flags.get_mut() |= ModuleFlags::TEXT_REL;
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

/// Contains TLS information the [`Module`].
#[derive(Debug)]
pub struct ModuleTls {
    init: usize,
    init_size: usize,
    size: usize,
    align: usize,
}

impl ModuleTls {
    pub fn init(&self) -> usize {
        self.init
    }

    pub fn init_size(&self) -> usize {
        self.init_size
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn align(&self) -> usize {
        self.align
    }
}

/// Contains exception handling information for [`Module`].
#[derive(Debug)]
pub struct ModuleEh {
    header: usize,
    header_size: usize,
    frame: usize,
    frame_size: usize,
}

impl ModuleEh {
    pub fn header(&self) -> usize {
        self.header
    }

    pub fn header_size(&self) -> usize {
        self.header_size
    }

    pub fn frame(&self) -> usize {
        self.frame
    }

    pub fn frame_size(&self) -> usize {
        self.frame_size
    }
}

bitflags! {
    /// Flags for [`Module`].
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ModuleFlags: u16 {
        const MAIN_PROG = 0x0001;
        const TEXT_REL = 0x0002;
        const JMPSLOTS_DONE = 0x0004; // TODO: This seems incorrect.
        const TLS_DONE = 0x0008;
        const INIT_SCANNED = 0x0010;
        const ON_FINI_LIST = 0x0020;
        const DAG_INITED = 0x0040;
        const UNK1 = 0x0100; // TODO: Rename this.
        const UNK2 = 0x0200; // TODO: Rename this.
        const UNK3 = 0x0400; // TODO: Rename this.
        const UNK4 = 0x0800; // TODO: It seems like this is actually JMPSLOTS_DONE.
        const UNK5 = 0x1000; // TODO: Rename this.
    }
}

impl Display for ModuleFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// An implementation of `Needed_Entry`.
#[derive(Debug)]
pub struct NeededModule {
    name: String,
}
