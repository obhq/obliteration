use self::fs::Fs;
use self::fs::MountPoint;
use self::lifter::LiftedModule;
use self::llvm::Llvm;
use self::memory::MemoryManager;
use self::module::Module;
use clap::Parser;
use elf::dynamic::RelocationInfo;
use elf::dynamic::SymbolInfo;
use elf::Elf;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::VecDeque;
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

    if let Err(e) = fs.mount("/", MountPoint::new(args.system.clone())) {
        error!(e, "Mount failed");
        return false;
    }

    info!("Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount("/mnt/app0", MountPoint::new(args.game)) {
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

    // Load dependencies.
    let mut modules = HashMap::from([(String::from(""), eboot)]);

    if let Some(dynamic) = modules[""].image().dynamic_linking() {
        let mut deps: VecDeque<String> = dynamic.dependencies().iter().map(|m| m.clone()).collect();

        while let Some(dep) = deps.pop_front() {
            use std::collections::hash_map::Entry;

            // Remove file extension.
            let name = match dep.rfind('.').map(|i| (&dep[..i]).to_owned()) {
                Some(v) if !v.is_empty() => v,
                _ => {
                    error!("Invalid module file name: {dep}");
                    return false;
                }
            };

            // Check if already loaded.
            let entry = match modules.entry(name) {
                Entry::Occupied(_) => continue,
                Entry::Vacant(e) => e,
            };

            // Load the module.
            let module = match load_module(&fs, mm.clone(), ModuleName::Search(entry.key())) {
                Some(v) => v,
                None => return false,
            };

            if let Some(dynamic) = module.image().dynamic_linking() {
                deps.extend(dynamic.dependencies().iter().map(|m| m.clone()));
            }

            entry.insert(module);
        }
    }

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

            match fs.get(name) {
                Ok(v) => match v {
                    fs::Item::Directory(_) => {
                        error!("Path to {} is a directory.", name);
                        return None;
                    }
                    fs::Item::File(v) => v,
                },
                Err(e) => {
                    error!(e, "Getting failed");
                    return None;
                }
            }
        }
        ModuleName::Search(name) => {
            info!("Looking for {name}.");

            'search: {
                // Try sce_module inside game directory first.
                match fs.get(&format!("/mnt/app0/sce_module/{name}.prx")) {
                    Ok(v) => match v {
                        fs::Item::Directory(_) => {
                            // FIXME: Right now FS will treat non-existent file as a directory.
                        }
                        fs::Item::File(v) => break 'search v,
                    },
                    Err(e) => {
                        error!(e, "Looking failed");
                        return None;
                    }
                }

                // Next try system/common/lib.
                match fs.get(&format!("/system/common/lib/{name}.sprx")) {
                    Ok(v) => match v {
                        fs::Item::Directory(_) => {
                            // FIXME: Right now FS will treat non-existent file as a directory.
                        }
                        fs::Item::File(v) => break 'search v,
                    },
                    Err(e) => {
                        error!(e, "Looking failed");
                        return None;
                    }
                }

                // Next try system/priv/lib.
                match fs.get(&format!("/system/priv/lib/{name}.sprx")) {
                    Ok(v) => match v {
                        fs::Item::Directory(_) => {
                            // FIXME: Right now FS will treat non-existent file as a directory.
                        }
                        fs::Item::File(v) => break 'search v,
                    },
                    Err(e) => {
                        error!(e, "Looking failed");
                        return None;
                    }
                }

                error!("Cannot find {name}.");
                return None;
            }
        }
    };

    // Open the module without allocating a virtual file descriptor.
    let virtual_path = file.virtual_path().to_owned();

    info!("Loading {virtual_path}.");

    let file = match File::open(file.path()) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Open failed");
            return None;
        }
    };

    // Load the module.
    let elf = match Elf::open(&virtual_path, file) {
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
        for (i, m) in dynamic.needed_modules().iter().enumerate() {
            info!("========== Needed module #{} ==========", i);
            info!("ID           : {}", m.id());
            info!("Name         : {}", m.name());
            info!("Major version: {}", m.version_major());
            info!("Minor version: {}", m.version_minor());
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
