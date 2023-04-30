use crate::fs::path::VPath;
use crate::fs::path::VPathBuf;
use crate::fs::Fs;
use crate::lifter::LiftedModule;
use crate::llvm::Llvm;
use crate::memory::MemoryManager;
use crate::module::Module;
use crate::module::ModuleManager;
use clap::Parser;
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;

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
    let mut loaded = Vec::new();

    info!("Loading eboot.bin.");

    let eboot = match modules.load_eboot() {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return false;
        }
    };

    loaded.push(eboot.clone());

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

            loaded.push(m);
        }
    }

    info!("{} module(s) has been loaded successfully.", loaded.len());

    // Apply module relocations.
    for module in loaded {
        info!("Applying relocation entries on {}.", module.image().name());

        if let Err(e) = unsafe { module.apply_relocs(|h, n| modules.resolve_symbol(h, n)) } {
            error!(e, "Applying failed");
            return false;
        }
    }

    // Get dependency chain.
    info!("Getting eboot.bin dependency chain.");

    let deps = match modules.get_deps(&eboot) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Getting failed");
            return false;
        }
    };

    // Lift the loaded modules.
    for module in [eboot].iter().chain(deps.iter()).rev() {
        // Lift the module.
        info!("Lifting {}.", module.image().name());

        match LiftedModule::lift(&llvm, module.clone()) {
            Ok(_) => {} // TODO: Store the lifted module somewhere.
            Err(e) => {
                error!(e, "Lifting failed");
                return false;
            }
        }
    }

    true
}

fn print_module(module: &Module) {
    // Image type.
    let image = module.image();

    if image.self_segments().is_some() {
        info!("Image type    : SELF");
    } else {
        info!("Image type    : ELF");
    }

    // Dynamic linking.
    if let Some(dynamic) = image.dynamic_linking() {
        let i = dynamic.module_info();

        info!("Module name   : {}", i.name());
        info!("Major version : {}", i.version_major());
        info!("Minor version : {}", i.version_minor());

        if let Some(f) = dynamic.flags() {
            info!("Module flags  : {f}");
        }

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
        module.memory().addr(),
        module.memory().addr() + module.memory().len()
    );

    info!(
        "Entry address : {:#018x}",
        module.memory().addr() + module.image().entry_addr()
    );

    for s in module.memory().segments().iter() {
        let addr = module.memory().addr() + s.start();

        info!(
            "Program {} is mapped to {:#018x}:{:#018x} with {}.",
            s.program(),
            addr,
            addr + s.len(),
            module.image().programs()[s.program()].flags(),
        );
    }
}
