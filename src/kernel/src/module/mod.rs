pub use memory::*;
pub use module::*;
pub use workspace::*;

use crate::fs::path::{VPath, VPathBuf};
use crate::fs::{Fs, FsItem};
use crate::memory::MemoryManager;
use elf::Elf;
use std::collections::{HashMap, HashSet};
use std::fs::{read_dir, File};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use thiserror::Error;

mod memory;
mod module;
mod workspace;

/// Manage all loaded modules.
pub struct ModuleManager<'a> {
    fs: &'a Fs,
    mm: &'a MemoryManager,
    module_workspace: usize,
    available: HashMap<String, Vec<VPathBuf>>, // Key is module name.
    loaded: RwLock<HashMap<VPathBuf, Arc<Module<'a>>>>,
    next_id: AtomicU64,
}

impl<'a> ModuleManager<'a> {
    pub const EBOOT_PATH: &str = "/mnt/app0/eboot.bin";

    pub fn new(fs: &'a Fs, mm: &'a MemoryManager, module_workspace: usize) -> Self {
        let mut m = Self {
            fs,
            mm,
            module_workspace,
            available: HashMap::new(),
            loaded: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        };

        m.update_available("/mnt/app0/sce_module".try_into().unwrap());
        m.update_available("/system/common/lib".try_into().unwrap());
        m.update_available("/system/priv/lib".try_into().unwrap());

        m
    }

    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    pub fn get_eboot(&self) -> Arc<Module<'a>> {
        let key: &VPath = Self::EBOOT_PATH.try_into().unwrap();
        let loaded = self.loaded.read().unwrap();

        match loaded.get(key) {
            Some(v) => v.clone(),
            None => panic!("eboot.bin is not loaded."),
        }
    }

    pub fn get_mod(&self, path: &VPath) -> Option<Arc<Module<'a>>> {
        self.loaded.read().unwrap().get(path).cloned()
    }

    pub fn for_each<F, E>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&Arc<Module<'a>>) -> Result<(), E>,
    {
        let loaded = self.loaded.read().unwrap();

        for (_, m) in loaded.deref() {
            f(m)?;
        }

        Ok(())
    }

    /// Recursive get the dependencies of `target`. The return value is ordered by the dependency
    /// chain, which mean the most common module will be the last item.
    pub fn get_deps(&self, target: &Module) -> Result<Vec<Arc<Module>>, DependencyChainError> {
        // Check if the module is a dynamic module.
        let dynamic = match target.image().dynamic_linking() {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        // Collect dependencies.
        let loaded = self.loaded.read().unwrap();
        let mut deps: HashMap<&VPathBuf, (usize, Arc<Module>)> = HashMap::new();
        let mut current: Vec<&str> = dynamic.dependencies().values().map(|m| m.name()).collect();
        let mut next: HashSet<&str> = HashSet::new();

        for level in 0.. {
            if current.is_empty() {
                break;
            }

            for dep in current.drain(..) {
                // Get module path.
                let paths = match self.available.get(dep) {
                    Some(v) => v,
                    None => return Err(DependencyChainError::NoModule(dep.to_owned())),
                };

                for path in paths {
                    use std::collections::hash_map::Entry;

                    // Check if module exists in the chain.
                    let entry = match deps.entry(path) {
                        Entry::Occupied(mut e) => {
                            e.get_mut().0 = level;
                            continue;
                        }
                        Entry::Vacant(e) => e,
                    };

                    // Get loaded module.
                    let module = match loaded.get(path) {
                        Some(v) => v,
                        None => return Err(DependencyChainError::NotLoaded(path.clone())),
                    };

                    entry.insert((level, module.clone()));

                    // Get module dependencies.
                    let dynamic = match module.image().dynamic_linking() {
                        Some(v) => v,
                        None => continue,
                    };

                    for dep in dynamic.dependencies().values() {
                        next.insert(dep.name());
                    }
                }
            }

            current.extend(next.drain());
        }

        // Create dependency chain.
        let mut chain: Vec<(usize, Arc<Module>)> = deps.into_values().collect();

        chain.sort_unstable_by_key(|i| i.0);

        Ok(chain.into_iter().map(|i| i.1).collect())
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
        let module = Arc::new(self.load(&path, self.module_workspace)?);

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
            let module = Arc::new(self.load(file, self.module_workspace)?);

            loaded.insert(file.clone(), module.clone());
            modules.push(module);
        }

        Ok(modules)
    }

    /// `name` is a normalized name (e.g. M0z6Dr6TNnM#libkernel#libkernel).
    pub fn resolve_symbol(&self, hash: u32, name: &str) -> Result<usize, ResolveSymbolError> {
        // Get module name.
        let module = match name.splitn(3, '#').nth(2) {
            Some(v) => v,
            None => return Err(ResolveSymbolError::InvalidName),
        };

        // Get module file.
        let files = match self.available.get(module) {
            Some(v) => v,
            None => return Err(ResolveSymbolError::InvalidModule),
        };

        // Lookup symbol from loaded modules.
        let loaded = self.loaded.read().unwrap();

        for file in files {
            // Get module.
            let module = match loaded.get(file) {
                Some(v) => v,
                None => return Err(ResolveSymbolError::NotLoaded),
            };

            // Skip if the module is not dynamic module.
            let dynamic = match module.image().dynamic_linking() {
                Some(v) => v,
                None => continue,
            };

            // Lookup.
            if let Some(sym) = dynamic.lookup_symbol(hash, name) {
                return Ok(module.memory().addr() + sym.value());
            }
        }

        Err(ResolveSymbolError::NotFound)
    }

    fn load(&self, path: &VPath, workspace: usize) -> Result<Module<'a>, LoadError> {
        // Get the module.
        let file = match self.fs.get(path) {
            Some(v) => match v {
                FsItem::Directory(_) => panic!("{path} is a directory."),
                FsItem::File(v) => v,
            },
            None => panic!("{path} does not exist."),
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
        Module::load(
            self.next_id.fetch_add(1, Ordering::Relaxed),
            elf,
            self.mm,
            workspace,
        )
    }

    fn update_available(&mut self, from: &VPath) {
        use std::collections::hash_map::Entry;

        // Get target directory.
        let dir = match self.fs.get(from) {
            Some(v) => match v {
                FsItem::Directory(v) => v,
                FsItem::File(_) => {
                    panic!("{from} was expected to be a directory but it is a file.")
                }
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

/// Represents the errors for [`ModuleManager::load_eboot()`] and [`ModuleManager::load_lib()`].
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("the specified module was not found")]
    NotFound,

    #[error("program #{0} has zero size in memory")]
    ZeroLenProgram(usize),

    #[error("there are no mappable programs")]
    NoMappablePrograms,

    #[error("program #{0} has address overlapped with another program")]
    ProgramAddressOverlapped(usize),

    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] crate::memory::MmapError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] elf::ReadProgramError),

    #[error("cannot protect memory")]
    ProtectionMemoryFailed(#[source] crate::memory::MprotectError),
}

/// Represents the error for symbol resolving.
#[derive(Debug, Error)]
pub enum ResolveSymbolError {
    #[error("invalid name")]
    InvalidName,

    #[error("invalid module")]
    InvalidModule,

    #[error("module is not loaded")]
    NotLoaded,

    #[error("not found")]
    NotFound,
}

/// Represens the errors for dependency chain.
#[derive(Debug, Error)]
pub enum DependencyChainError {
    #[error("module {0} is not available")]
    NoModule(String),

    #[error("module {0} is not loaded")]
    NotLoaded(VPathBuf),
}
