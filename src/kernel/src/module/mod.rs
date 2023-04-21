use crate::fs::path::{VPath, VPathBuf};
use crate::fs::{Fs, FsItem};
use crate::memory::MemoryManager;
use elf::dynamic::{RelocationInfo, SymbolInfo};
use elf::{Elf, ProgramFlags, ProgramType};
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::{Read, Seek};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Manage all loaded modules.
pub struct ModuleManager<'a> {
    fs: &'a Fs,
    mm: &'a MemoryManager,
    available: HashMap<String, Vec<VPathBuf>>, // Key is module name.
    loaded: RwLock<HashMap<VPathBuf, Arc<Module<'a>>>>,
}

impl<'a> ModuleManager<'a> {
    pub const EBOOT_PATH: &str = "/mnt/app0/eboot.bin";

    pub fn new(fs: &'a Fs, mm: &'a MemoryManager) -> Self {
        let mut m = Self {
            fs,
            mm,
            available: HashMap::new(),
            loaded: RwLock::new(HashMap::new()),
        };

        m.update_available("/mnt/app0/sce_module".try_into().unwrap());
        m.update_available("/system/common/lib".try_into().unwrap());
        m.update_available("/system/priv/lib".try_into().unwrap());

        m
    }

    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// This function only load eboot.bin without its dependencies into the memory, no relocation is
    /// applied.
    pub fn load_eboot(&self) -> Result<Arc<Module>, LoadError> {
        // Check if already loaded.
        let path = VPathBuf::try_from(Self::EBOOT_PATH).unwrap();
        let mut loaded = self.loaded.write().unwrap();

        if loaded.contains_key(&path) {
            panic!("{path} is already loaded.");
        }

        // Load the module.
        let module = Arc::new(self.load(&path)?);

        loaded.insert(path, module.clone());

        Ok(module)
    }

    /// Load only the specified module without its dependencies into the memory, no relocation is
    /// applied. Returns only the modules that was loaded by this call, which is zero if the module
    /// is already loaded.
    pub fn load_mod(&self, name: &str) -> Result<Vec<Arc<Module>>, LoadError> {
        let mut modules = Vec::new();

        // Map name to file.
        let files = match self.available.get(name) {
            Some(v) => v,
            None => return Err(LoadError::NotFound),
        };

        // Load all files.
        let mut loaded = self.loaded.write().unwrap();

        for file in files {
            // Check if already loaded.
            if loaded.get(file).is_some() {
                continue;
            }

            // Load the module.
            let module = Arc::new(self.load(&file)?);

            loaded.insert(file.clone(), module.clone());
            modules.push(module);
        }

        Ok(modules)
    }

    fn load(&self, path: &VPath) -> Result<Module<'a>, LoadError> {
        // Get the module.
        let file = match self.fs.get(path) {
            Some(v) => match v {
                FsItem::Directory(_) => panic!("{path} is a directory."),
                FsItem::File(v) => v,
            },
            None => panic!("{path} does not exists."),
        };

        // Open the module.
        let file = match File::open(file.path()) {
            Ok(v) => v,
            Err(e) => panic!("Cannot open {path}: {e}."),
        };

        // Load the module.
        let elf = match Elf::open(path, file) {
            Ok(v) => v,
            Err(e) => panic!("Cannot open SELF from {path}: {e}."),
        };

        // Map the module to the memory.
        Module::load(elf, self.mm)
    }

    fn update_available(&mut self, from: &VPath) {
        use std::collections::hash_map::Entry;

        // Get target directory.
        let dir = match self.fs.get(from) {
            Some(v) => match v {
                FsItem::Directory(v) => v,
                FsItem::File(_) => panic!("{from} is expected to be a directory but it is a file."),
            },
            None => return,
        };

        // Open the directlry.
        let items = match read_dir(dir.path()) {
            Ok(v) => v,
            Err(e) => panic!("Cannot open {}: {e}.", dir.path().display()),
        };

        // Enumerate files.
        for item in items {
            let item = match item {
                Ok(v) => v,
                Err(e) => panic!("Cannot read a file in {}: {e}.", dir.path().display()),
            };

            // Skip if a directory.
            let path = item.path();
            let meta = match std::fs::metadata(&path) {
                Ok(v) => v,
                Err(e) => panic!("Cannot get metadata of {}: {e}.", path.display()),
            };

            if meta.is_dir() {
                continue;
            }

            // Skip if not an (S)PRX file.
            match path.extension() {
                Some(ext) => {
                    if ext != "prx" && ext != "sprx" {
                        continue;
                    }
                }
                None => continue,
            }

            // Open the file.
            let file = match File::open(&path) {
                Ok(v) => v,
                Err(e) => panic!("Cannot open {}: {e}.", path.display()),
            };

            let elf = match Elf::open(path.to_string_lossy(), file) {
                Ok(v) => v,
                Err(e) => panic!("Cannot inspect {}: {e}.", path.display()),
            };

            // Get dynamic linking info.
            let dynamic = match elf.dynamic_linking() {
                Some(v) => v,
                None => panic!("{} is not a dynamic module.", path.display()),
            };

            // Get map entry.
            let info = dynamic.module_info();
            let list = match self.available.entry(info.name().to_owned()) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(Vec::new()),
            };

            // Get file name.
            let name = match item.file_name().into_string() {
                Ok(v) => v,
                Err(_) => panic!("{} has unsupported alphabet.", path.display()),
            };

            // Push virtual path to the list.
            let mut vpath = dir.virtual_path().to_owned();

            if let Err(e) = vpath.push(&name) {
                panic!("Cannot build a virtual path for {}: {e}.", path.display());
            }

            list.push(vpath);
        }
    }
}

/// Represents a loaded SELF in an unmodified state (no code lifting, etc.). That is, the same
/// representation as on PS4.
pub struct Module<'a> {
    image: Elf<File>,
    memory: Memory<'a>,
}

impl<'a> Module<'a> {
    fn load(mut image: Elf<File>, mm: &'a MemoryManager) -> Result<Self, LoadError> {
        // Map SELF to the memory.
        let mut memory = Memory::new(&image, mm)?;

        memory.load(|prog, buf| {
            if let Err(e) = image.read_program(prog, buf) {
                Err(LoadError::ReadProgramFailed(prog, e))
            } else {
                Ok(())
            }
        })?;

        memory.protect(&image)?;

        Ok(Self { image, memory })
    }

    pub fn image(&self) -> &Elf<File> {
        &self.image
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn apply_relocs(&self) -> Result<(), RelocError> {
        // Do nothing if the module is not dynamic linking.
        let dynamic = match self.image.dynamic_linking() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Apply relocation.
        for (i, reloc) in dynamic.relocation_entries().enumerate() {
            // Resolve the value.
            let value = match reloc.ty() {
                RelocationInfo::R_X86_64_64
                | RelocationInfo::R_X86_64_PC32
                | RelocationInfo::R_X86_64_GLOB_DAT
                | RelocationInfo::R_X86_64_DTPMOD64
                | RelocationInfo::R_X86_64_DTPOFF64
                | RelocationInfo::R_X86_64_TPOFF64
                | RelocationInfo::R_X86_64_DTPOFF32
                | RelocationInfo::R_X86_64_TPOFF32 => {
                    // Get target symbol.
                    let symbol = match dynamic.symbols().get(reloc.symbol()) {
                        Some(v) => v,
                        None => return Err(RelocError::InvalidSymbolIndex(i)),
                    };

                    // Check binding type.
                    match symbol.binding() {
                        SymbolInfo::STB_LOCAL => self.memory.addr() + symbol.value(),
                        SymbolInfo::STB_GLOBAL | SymbolInfo::STB_WEAK => {
                            // TODO: Resolve external symbol.
                            0
                        }
                        v => return Err(RelocError::UnknownSymbolBinding(i, v)),
                    }
                }
                RelocationInfo::R_X86_64_RELATIVE => 0,
                v => return Err(RelocError::UnknownRelocationType(i, v)),
            };

            // TODO: Apply the value.
        }

        // Apply Procedure Linkage Table relocation.
        for (i, reloc) in dynamic.plt_relocation().enumerate() {
            // Resolve the value.
            let value = match reloc.ty() {
                RelocationInfo::R_X86_64_JUMP_SLOT => {
                    // Get target symbol.
                    let symbol = match dynamic.symbols().get(reloc.symbol()) {
                        Some(v) => v,
                        None => return Err(RelocError::InvalidPltSymIndex(i)),
                    };

                    // Check binding type.
                    match symbol.binding() {
                        SymbolInfo::STB_LOCAL => self.memory.addr() + symbol.value(),
                        SymbolInfo::STB_GLOBAL | SymbolInfo::STB_WEAK => {
                            // TODO: Resolve external symbol.
                            0
                        }
                        v => return Err(RelocError::UnknownPltSymBinding(i, v)),
                    }
                }
                RelocationInfo::R_X86_64_RELATIVE => 0,
                v => return Err(RelocError::UnknownPltRelocType(i, v)),
            };

            // TODO: Apply the value.
        }

        Ok(())
    }
}

/// Represents a memory of the module.
pub struct Memory<'a> {
    mm: &'a MemoryManager,
    ptr: *mut u8,
    len: usize,
    segments: Vec<MemorySegment>,
}

impl<'a> Memory<'a> {
    fn new<I: Read + Seek>(elf: &Elf<I>, mm: &'a MemoryManager) -> Result<Self, LoadError> {
        use crate::memory::{MappingFlags, Protections};

        let programs = elf.programs();

        // Create segments from programs.
        let mut segments: Vec<MemorySegment> = Vec::with_capacity(programs.len());

        for (i, p) in programs.iter().enumerate() {
            let t = p.ty();

            if t == ProgramType::PT_LOAD || t == ProgramType::PT_SCE_RELRO {
                let s = MemorySegment {
                    start: p.addr(),
                    len: p.aligned_size(),
                    program: i,
                };

                if s.len == 0 {
                    return Err(LoadError::ZeroLenProgram(i));
                }

                segments.push(s);
            }
        }

        if segments.is_empty() {
            return Err(LoadError::NoMappablePrograms);
        }

        // Make sure no any segment is overlapped.
        let mut len = 0;

        segments.sort_unstable_by_key(|s| s.start);

        for s in &segments {
            if s.start < len {
                return Err(LoadError::ProgramAddressOverlapped(s.program));
            }

            len += s.len;
        }

        // Allocate pages.
        let ptr = match mm.mmap(
            0,
            len,
            Protections::CPU_READ | Protections::CPU_WRITE,
            MappingFlags::MAP_ANON | MappingFlags::MAP_PRIVATE,
            -1,
            0,
        ) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::MemoryAllocationFailed(len, e)),
        };

        Ok(Self {
            mm,
            ptr,
            len,
            segments,
        })
    }

    fn load<L, E>(&mut self, mut loader: L) -> Result<(), E>
    where
        L: FnMut(usize, &mut [u8]) -> Result<(), E>,
    {
        for seg in &self.segments {
            // Get destination buffer.
            let ptr = unsafe { self.ptr.add(seg.start) };
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr, seg.len) };

            // Invoke loader.
            loader(seg.program, dst)?;
        }

        Ok(())
    }

    fn protect<I: Read + Seek>(&mut self, elf: &Elf<I>) -> Result<(), LoadError> {
        use crate::memory::Protections;

        let progs = elf.programs();

        for seg in &self.segments {
            // Derive protections from program flags.
            let flags = progs[seg.program].flags();
            let mut prot = Protections::NONE;

            if flags.contains(ProgramFlags::EXECUTE) {
                prot |= Protections::CPU_EXEC;
            }

            if flags.contains(ProgramFlags::READ) {
                prot |= Protections::CPU_READ;
            }

            if flags.contains(ProgramFlags::WRITE) {
                prot |= Protections::CPU_WRITE;
            }

            // Change protection.
            let addr = unsafe { self.ptr.add(seg.start) };

            if let Err(e) = self.mm.mprotect(addr, seg.len, prot) {
                return Err(LoadError::ChangeProtectionFailed(seg.program, e));
            }
        }

        Ok(())
    }

    pub fn addr(&self) -> usize {
        self.ptr as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn segments(&self) -> &[MemorySegment] {
        self.segments.as_ref()
    }
}

impl<'a> AsRef<[u8]> for Memory<'a> {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<'a> Drop for Memory<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.mm.munmap(self.ptr, self.len) {
            panic!(
                "Failed to unmap {} bytes starting at {:p}: {}.",
                self.len, self.ptr, e
            );
        }
    }
}

/// Contains information for a segment in [`Memory`].
pub struct MemorySegment {
    start: usize,
    len: usize,
    program: usize,
}

impl MemorySegment {
    /// Gets the offset within the module memory of this segment.
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Gets the corresponding index of SELF program.
    pub fn program(&self) -> usize {
        self.program
    }
}

/// Represents the errors for [`ModuleManager::load_eboot()`] and [`ModuleManager::load_lib()`].
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("the specified module is not found")]
    NotFound,

    #[error("program #{0} has zero size in the memory")]
    ZeroLenProgram(usize),

    #[error("no any mappable programs")]
    NoMappablePrograms,

    #[error("program #{0} has address overlapped with the other program")]
    ProgramAddressOverlapped(usize),

    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] crate::memory::MmapError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] elf::ReadProgramError),

    #[error("cannot change protection for mapped program #{0}")]
    ChangeProtectionFailed(usize, #[source] crate::memory::MprotectError),
}

/// Represents the errors for [`Module::apply_relocs()`].
#[derive(Debug, Error)]
pub enum RelocError {
    #[error("unknown relocation type {1:#010x} on entry {0}")]
    UnknownRelocationType(usize, u32),

    #[error("invalid symbol index on entry {0}")]
    InvalidSymbolIndex(usize),

    #[error("unknown symbol binding type {1} on entry {0}")]
    UnknownSymbolBinding(usize, u8),

    #[error("unknown PLT relocation type {1:#010x} on entry {0}")]
    UnknownPltRelocType(usize, u32),

    #[error("invalid symbol index on PLT entry {0}")]
    InvalidPltSymIndex(usize),

    #[error("unknown symbol binding type {1} on PLT entry {0}")]
    UnknownPltSymBinding(usize, u8),
}
