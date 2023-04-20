use crate::fs::path::VPath;
use crate::fs::path::VPathBuf;
use crate::fs::Fs;
use crate::lifter::LiftedModule;
use crate::llvm::Llvm;
use crate::memory::MemoryManager;
use crate::module::ModuleManager;
use clap::Parser;
use elf::dynamic::RelocationInfo;
use elf::dynamic::SymbolInfo;
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;

use self::module::Module;

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
    let fs = Fs::new();

    info!("Mounting / to {}.", args.system.display());

    if let Err(e) = fs.mount(VPathBuf::new(), args.system) {
        error!(e, "Mount failed");
        return false;
    }

    info!("Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount(VPath::new("/mnt/app0").unwrap(), args.game) {
        error!(e, "Mount failed");
        return false;
    }

    // Initialize memory manager.
    info!("Initializing memory manager.");

    let mm = MemoryManager::new();

    info!("Page size is: {}.", mm.page_size());
    info!(
        "Allocation granularity is: {}.",
        mm.allocation_granularity()
    );

    // Initialize the module manager.
    info!("Initializing module manager.");

    let modules = ModuleManager::new(&fs, &mm);

    info!("{} modules is available.", modules.available_count());

    // Load eboot.bin.
    info!("Loading eboot.bin.");

    let eboot = match modules.load_eboot() {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return false;
        }
    };

    print_module(&eboot);

    // Load dependencies.
    info!("Loading eboot.bin dependencies.");

    let mut deps = match eboot.image().dynamic_linking() {
        Some(dynamic) => dynamic
            .dependencies()
            .values()
            .map(|m| m.name().to_owned())
            .collect(),
        None => VecDeque::new(),
    };

    while let Some(name) = deps.pop_front() {
        // Load the module.
        let mods = match modules.load_mod(&name) {
            Ok(v) => v,
            Err(e) => {
                error!(e, "Cannot load {name}");
                return false;
            }
        };

        for m in mods {
            // Print module information.
            info!("Module {name} is mapped to {}.", m.image().name());
            print_module(&m);

            // Add dependencies.
            let dynamic = match m.image().dynamic_linking() {
                Some(v) => v,
                None => continue,
            };

            for dep in dynamic.dependencies().values() {
                deps.push_back(dep.name().to_owned());
            }
        }
    }

    info!(
        "{} module(s) has been loaded successfully.",
        modules.loaded_count()
    );

    // Apply module relocations.
    for module in [&eboot] {
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
    for module in [eboot] {
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

fn print_module(m: &Module) {
    if m.image().self_segments().is_some() {
        info!("Image type    : SELF");
    } else {
        info!("Image type    : ELF");
    }

    if let Some(dynamic) = m.image().dynamic_linking() {
        let i = dynamic.module_info();

        info!("Module name   : {}", i.name());
        info!("Major version : {}", i.version_major());
        info!("Minor version : {}", i.version_minor());

        for m in dynamic.dependencies().values() {
            info!(
                "Needed module : {} v{}.{}",
                m.name(),
                m.version_major(),
                m.version_minor()
            );
        }
    }

    info!(
        "Memory address: {:#018x}:{:#018x}",
        m.memory().addr(),
        m.memory().addr() + m.memory().len()
    );

    info!(
        "Entry address : {:#018x}",
        m.memory().addr() + m.image().entry_addr()
    );

    for s in m.memory().segments().iter() {
        let addr = m.memory().addr() + s.start();

        info!(
            "Program {} is mapped to {:#018x}:{:#018x} with {:?}.",
            s.program(),
            addr,
            addr + s.len(),
            m.image().programs()[s.program()].flags(),
        );
    }
}
