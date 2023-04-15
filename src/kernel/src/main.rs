use crate::fs::path::Vpath;
use crate::fs::path::VpathBuf;
use crate::fs::Fs;
use crate::lifter::LiftedModule;
use crate::llvm::Llvm;
use crate::memory::MemoryManager;
use crate::module::Module;
use clap::Parser;
use elf::dynamic::RelocationInfo;
use elf::dynamic::SymbolInfo;
use elf::Elf;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

mod errno;
mod fs;
mod lifter;
mod llvm;
mod log;
mod memory;
mod module;

#[derive(Parser, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Args {
    #[arg(long)]
    system: PathBuf,

    #[arg(long)]
    game: PathBuf,

    #[arg(long)]
    debug_dump: PathBuf,

    #[arg(long)]
    clear_debug_dump: bool,
}

fn main() {
    std::process::exit(if run() { 0 } else { 1 });
}

fn run() -> bool {
    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = match File::open(".kernel-debug") {
            Ok(v) => v,
            Err(e) => {
                error!(e, "Failed to open .kernel-debug");
                return false;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                error!(e, "Failed to read .kernel-debug");
                return false;
            }
        }
    } else {
        Args::parse()
    };

    // Remove previous debug dump.
    if args.clear_debug_dump {
        if let Err(e) = std::fs::remove_dir_all(&args.debug_dump) {
            if e.kind() != std::io::ErrorKind::NotFound {
                error!(e, "Failed to remove {}", args.debug_dump.display());
                return false;
            }
        }
    }

    // Show basic infomation.
    info!("Starting Obliteration kernel.");
    info!("Debug dump directory is: {}.", args.debug_dump.display());

    // Initialize LLVM.
    let llvm = Llvm::new();

    // Initialize filesystem.
    let fs = Arc::new(Fs::new());

    info!("Mounting / to {}.", args.system.display());

    if let Err(e) = fs.mount(VpathBuf::new(), args.system) {
        error!(e, "Mount failed");
        return false;
    }

    info!("Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount(Vpath::new("/mnt/app0").unwrap(), args.game) {
        error!(e, "Mount failed");
        return false;
    }

    // Initialize memory manager.
    info!("Initializing memory manager.");

    let mm = Arc::new(MemoryManager::new());

    info!("Page size is: {}.", mm.page_size());
    info!(
        "Allocation granularity is: {}.",
        mm.allocation_granularity()
    );

    // Load eboot.bin.
    let eboot = match load_module(&fs, mm.clone(), ModuleName::Absolute("/mnt/app0/eboot.bin")) {
        Some(v) => v,
        None => return false,
    };

    // TODO: Load dependencies.
    let mut modules = HashMap::from([(String::from(""), eboot)]);

    info!("{} module(s) has been loaded successfully.", modules.len());

    // Apply module relocations.
    for (_, module) in &modules {
        // Skip if the module is not dynamic linking.
        let dynamic = match module.image().dynamic_linking() {
            Some(v) => v,
            None => continue,
        };

        // Apply relocations.
        info!("Applying relocation entries on {}.", module.image().name());

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
                        None => {
                            error!("Invalid symbol index on entry {i}.");
                            return false;
                        }
                    };

                    // Check binding type.
                    match symbol.binding() {
                        SymbolInfo::STB_LOCAL => module.memory().addr() + symbol.value(),
                        SymbolInfo::STB_GLOBAL | SymbolInfo::STB_WEAK => {
                            info!("Linking symbol: {}", symbol.name());

                            // TODO: Resolve external symbol.
                            0
                        }
                        v => {
                            error!("Unknown symbol binding type {v} on entry {i}.");
                            return false;
                        }
                    }
                }
                RelocationInfo::R_X86_64_RELATIVE => 0,
                v => {
                    error!("Unknown relocation type {v:#010x} on entry {i}.");
                    return false;
                }
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
                        None => {
                            error!("Invalid symbol index on PLT entry {i}.");
                            return false;
                        }
                    };

                    // Check binding type.
                    match symbol.binding() {
                        SymbolInfo::STB_LOCAL => module.memory().addr() + symbol.value(),
                        SymbolInfo::STB_GLOBAL | SymbolInfo::STB_WEAK => {
                            info!("Linking PLT symbol: {}", symbol.name());

                            // TODO: Resolve external symbol.
                            0
                        }
                        v => {
                            error!("Unknown symbol binding type {v} on PLT entry {i}.");
                            return false;
                        }
                    }
                }
                RelocationInfo::R_X86_64_RELATIVE => 0,
                v => {
                    error!("Unknown PLT relocation type {v:#010x} on entry {i}.");
                    return false;
                }
            };

            // TODO: Apply the value.
        }
    }

    // Lift the loaded modules.
    for (_, module) in modules {
        // Lift the module.
        info!("Lifting {}.", module.image().name());

        match LiftedModule::lift(&llvm, module) {
            Ok(_) => {} // TODO: Store the lifted module somewhere.
            Err(e) => {
                error!(e, "Lifting failed");
                return false;
            }
        }
    }

    true
}

fn load_module(fs: &Fs, mm: Arc<MemoryManager>, name: ModuleName) -> Option<Module<File>> {
    // Get the module.
    let file = match name {
        ModuleName::Absolute(name) => {
            info!("Getting {}.", name);

            match fs.get(name.try_into().unwrap()) {
                Some(v) => match v {
                    fs::Item::Directory(_) => {
                        error!("Path to {} is a directory.", name);
                        return None;
                    }
                    fs::Item::File(v) => v,
                },
                None => {
                    error!("{name} does not exists.");
                    return None;
                }
            }
        }
        ModuleName::Search(name) => {
            let mut file = None;

            info!("Looking for {name}.");

            for path in (LibrarySearchPaths { name, next: 0 }) {
                if let Some(v) = fs.get(&path) {
                    match v {
                        fs::Item::Directory(_) => {
                            error!("Path to {path} is a directory.");
                            return None;
                        }
                        fs::Item::File(v) => {
                            file = Some(v);
                            break;
                        }
                    }
                }
            }

            match file {
                Some(v) => v,
                None => {
                    error!("Cannot find {name}.");
                    return None;
                }
            }
        }
    };

    // Open the module without allocating a virtual file descriptor.
    let virtual_path = file.virtual_path();

    info!("Loading {virtual_path}.");

    let file = match File::open(file.path()) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Open failed");
            return None;
        }
    };

    // Load the module.
    let elf = match Elf::open(virtual_path, file) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return None;
        }
    };

    info!("Entry address     : {:#018x}", elf.entry_addr());

    if let Some(dynamic) = elf.dynamic_linking() {
        let i = dynamic.module_info();

        info!("Module name       : {}", i.name());
        info!("Major version     : {}", i.version_major());
        info!("Minor version     : {}", i.version_minor());
    }

    if let Some(segments) = elf.self_segments() {
        info!("Image type        : SELF");

        for (i, s) in segments.iter().enumerate() {
            info!("============= Segment #{} =============", i);
            info!("Flags            : {:?}", s.flags());
            info!("Offset           : {}", s.offset());
            info!("Compressed size  : {}", s.compressed_size());
            info!("Decompressed size: {}", s.decompressed_size());
        }
    } else {
        info!("Image type        : ELF");
    }

    if let Some(dynamic) = elf.dynamic_linking() {
        for m in dynamic.dependencies().values() {
            info!(
                "Needed module: {} v{}.{}",
                m.name(),
                m.version_major(),
                m.version_minor()
            );
        }
    }

    // Map the module to the memory.
    info!("Mapping {}.", virtual_path);

    let module = match Module::load(elf, mm) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Map failed");
            return None;
        }
    };

    info!("Memory address: {:#018x}", module.memory().addr());
    info!("Memory size   : {:#018x}", module.memory().len());

    for (i, s) in module.memory().segments().iter().enumerate() {
        info!("============= Segment #{} =============", i);
        info!("Address: {:#018x}", module.memory().addr() + s.start());
        info!("Size   : {:#018x}", s.len());
        info!("Program: {}", s.program());
    }

    Some(module)
}

enum ModuleName<'a> {
    Absolute(&'a str),
    Search(&'a str),
}

struct LibrarySearchPaths<'a> {
    name: &'a str,
    next: usize,
}

impl<'a> Iterator for LibrarySearchPaths<'a> {
    type Item = VpathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        let path = match self.next {
            0 => format!("/mnt/app0/sce_module/{}.prx", self.name),
            1 => format!("/system/common/lib/{}.sprx", self.name),
            2 => format!("/system/priv/lib/{}.sprx", self.name),
            _ => return None,
        };

        self.next += 1;

        Some(path.try_into().unwrap())
    }
}
