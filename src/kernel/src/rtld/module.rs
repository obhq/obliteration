use super::{MapError, Memory};
use crate::ee::native::{NativeEngine, RawFn};
use crate::fs::{VFile, VPath, VPathBuf};
use crate::log::{print, LogEntry};
use crate::process::VProc;
use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use elf::{
    DynamicFlags, DynamicTag, Elf, FileInfo, FileType, LibraryFlags, LibraryInfo, ModuleInfo,
    Program, Symbol,
};
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::fmt::{Display, Formatter};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.h#L147.
#[derive(Debug)]
pub struct Module {
    ee: Arc<NativeEngine>,
    id: u32,
    init: Option<usize>,
    entry: Option<usize>,
    fini: Option<usize>,
    tls_index: u32,              // tlsindex
    tls_offset: Gutex<usize>,    // tlsoffset
    tls_info: Option<ModuleTls>, // tlsinit + tlsinitsize + tlssize + tlsalign
    eh_info: Option<ModuleEh>,
    proc_param: Option<(usize, usize)>,
    mod_param: Option<usize>,
    sdk_ver: u32,
    flags: Gutex<ModuleFlags>,
    names: Vec<String>,
    dag_static: Gutex<Vec<Arc<Self>>>,  // dagmembers
    dag_dynamic: Gutex<Vec<Arc<Self>>>, // dldags
    needed: Vec<NeededModule>,
    modules: Vec<ModuleInfo>,
    libraries: Vec<LibraryInfo>,
    fingerprint: [u8; 20],
    memory: Memory,
    relocated: Gutex<Vec<Option<Relocated>>>,
    file_info: Option<FileInfo>,
    path: VPathBuf,
    is_self: bool,
    file_type: FileType,
    programs: Vec<Program>,
    symbols: Vec<Symbol>,
    ref_count: Gutex<u32>,
}

impl Module {
    pub(super) fn map<N: Into<String>>(
        ee: &Arc<NativeEngine>,
        proc: &Arc<VProc>,
        mut image: Elf<VFile>,
        base: usize,
        mem_name: N,
        id: u32,
        names: Vec<String>,
        tls_index: u32,
    ) -> Result<Self, MapError> {
        // Map the image to the memory.
        let memory = Memory::new(proc, &image, base, mem_name)?;

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
        let entry = image.entry_addr();

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

        let mod_param = image
            .mod_param()
            .map(|i| image.program(i).unwrap())
            .map(|p| base + p.addr());

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
        let gg = GutexGroup::new();
        let mut module = Self {
            ee: ee.clone(),
            id,
            init: None,
            entry,
            fini: None,
            tls_index,
            tls_offset: gg.spawn(0),
            tls_info,
            eh_info,
            proc_param,
            mod_param,
            sdk_ver,
            flags: gg.spawn(ModuleFlags::IS_NEW),
            names,
            dag_static: gg.spawn(Vec::new()),
            dag_dynamic: gg.spawn(Vec::new()),
            needed: Vec::new(),
            modules: Vec::new(),
            libraries: Vec::new(),
            fingerprint: [0; 20],
            memory,
            relocated: gg.spawn(Vec::new()),
            file_info: None,
            path: path.try_into().unwrap(),
            is_self,
            file_type,
            programs,
            symbols,
            ref_count: gg.spawn(1),
        };

        if let Some(info) = file_info {
            module.digest_dynamic(base, &info)?;
            module.relocated = gg.spawn(
                std::iter::repeat_with(|| None)
                    .take(info.reloc_count() + info.plt_count())
                    .collect(),
            );
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

    pub fn tls_offset(&self) -> GutexReadGuard<'_, usize> {
        self.tls_offset.read()
    }

    pub fn tls_offset_mut(&self) -> GutexWriteGuard<'_, usize> {
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

    pub fn mod_param(&self) -> Option<usize> {
        self.mod_param
    }

    pub fn sdk_ver(&self) -> u32 {
        self.sdk_ver
    }

    pub fn flags(&self) -> GutexReadGuard<'_, ModuleFlags> {
        self.flags.read()
    }

    pub fn flags_mut(&self) -> GutexWriteGuard<'_, ModuleFlags> {
        self.flags.write()
    }

    pub fn names(&self) -> &[String] {
        self.names.as_ref()
    }

    pub fn dag_static(&self) -> GutexReadGuard<'_, Vec<Arc<Self>>> {
        self.dag_static.read()
    }

    pub fn dag_static_mut(&self) -> GutexWriteGuard<'_, Vec<Arc<Self>>> {
        self.dag_static.write()
    }

    pub fn dag_dynamic_mut(&self) -> GutexWriteGuard<'_, Vec<Arc<Self>>> {
        self.dag_dynamic.write()
    }

    pub fn modules(&self) -> &[ModuleInfo] {
        self.modules.as_ref()
    }

    pub fn libraries(&self) -> &[LibraryInfo] {
        self.libraries.as_ref()
    }

    pub fn fingerprint(&self) -> [u8; 20] {
        self.fingerprint
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn relocated_mut(&self) -> GutexWriteGuard<'_, Vec<Option<Relocated>>> {
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

    pub fn programs(&self) -> &[Program] {
        self.programs.as_ref()
    }

    pub fn symbols(&self) -> &[Symbol] {
        self.symbols.as_ref()
    }

    pub fn ref_count(&self) -> GutexReadGuard<'_, u32> {
        self.ref_count.read()
    }

    pub fn ref_count_mut(&self) -> GutexWriteGuard<'_, u32> {
        self.ref_count.write()
    }

    /// # Safety
    /// `off` must be a valid offset without base adjustment of a function in the memory of this
    /// module.
    pub unsafe fn get_function(self: &Arc<Self>, off: usize) -> Arc<RawFn> {
        self.ee
            .get_function(self, self.memory.addr() + self.memory.base() + off)
            .unwrap()
    }

    pub fn dump<P: AsRef<Path>>(&mut self, path: P) -> Result<(), std::io::Error> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.write_all(unsafe { self.memory.as_bytes() })
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
            writeln!(
                entry,
                "Entry address : {:#018x}",
                mem.addr() + mem.base() + v
            )
            .unwrap();
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
        let mut fingerprint_offset = None;

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
                DynamicTag::DT_SONAME => self.digest_soname(info, i, value)?,
                DynamicTag::DT_TEXTREL => *self.flags.get_mut() |= ModuleFlags::TEXT_REL,
                DynamicTag::DT_FLAGS => self.digest_flags(value)?,
                DynamicTag::DT_SCE_FINGERPRINT => {
                    fingerprint_offset = Some(u64::from_le_bytes(value) as usize)
                }
                DynamicTag::DT_SCE_MODULE_INFO | DynamicTag::DT_SCE_NEEDED_MODULE => {
                    self.digest_module_info(info, i, value)?;
                }
                DynamicTag::DT_SCE_EXPORT_LIB | DynamicTag::DT_SCE_IMPORT_LIB => {
                    self.digest_library_info(info, i, value, tag == DynamicTag::DT_SCE_EXPORT_LIB)?;
                }
                _ => continue,
            }
        }

        self.digest_fingerprint(info, fingerprint_offset.unwrap_or(0));

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
        self.init = Some(base + addr);
        Ok(())
    }

    fn digest_fini(&mut self, base: usize, value: [u8; 8]) -> Result<(), MapError> {
        // TODO: Apply checks from digest_dynamic on the PS4.
        let addr: usize = u64::from_le_bytes(value).try_into().unwrap();
        self.fini = Some(base + addr);
        Ok(())
    }

    fn digest_soname(&mut self, info: &FileInfo, i: usize, value: [u8; 8]) -> Result<(), MapError> {
        let name = u64::from_le_bytes(value);
        let name = match info.read_str(name.try_into().unwrap()) {
            Ok(v) => v,
            Err(e) => return Err(MapError::ReadNameFailed(i, e)),
        };

        self.names.push(name.to_owned());
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

    fn digest_fingerprint(&mut self, info: &FileInfo, offset: usize) {
        let fingerprint = info.read_fingerprint(offset);

        self.fingerprint = fingerprint;
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
        const MAINPROG = 0x0001;
        const TEXT_REL = 0x0002;
        const TLS_DONE = 0x0008;
        const INIT_SCANNED = 0x0010;
        const ON_FINI_LIST = 0x0020;
        const DAG_INITED = 0x0040;
        const IS_SYSTEM = 0x0100;
        const IS_NEW = 0x0200;
        const LIBC_FIOS = 0x0400;
        const JMPSLOTS_DONE = 0x0800;
        const NOT_GET_PROC = 0x1000;
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

/// Indicated a type of value in the relocation entry.
#[derive(Debug)]
pub enum Relocated {
    Executable(Arc<RawFn>),
    Data((Arc<Module>, usize)),
    Tls((Arc<Module>, usize)),
}
