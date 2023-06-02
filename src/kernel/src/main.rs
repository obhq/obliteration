use crate::fs::path::{VPath, VPathBuf};
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::memory::MemoryManager;
use crate::module::{Module, ModuleManager};
use crate::syscalls::Syscalls;
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;

mod disasm;
mod ee;
mod errno;
mod fs;
mod llvm;
mod log;
mod memory;
mod module;
mod syscalls;

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

    #[arg(long, short)]
    execution_engine: Option<ExecutionEngine>,
}

#[derive(Clone, ValueEnum, Deserialize)]
enum ExecutionEngine {
    Native,
    Llvm,
}

fn main() -> ExitCode {
    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = match File::open(".kernel-debug") {
            Ok(v) => v,
            Err(e) => {
                error!(e, "Failed to open .kernel-debug");
                return ExitCode::FAILURE;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                error!(e, "Failed to read .kernel-debug");
                return ExitCode::FAILURE;
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
                return ExitCode::FAILURE;
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
        return ExitCode::FAILURE;
    }

    info!("Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount(VPath::new("/mnt/app0").unwrap(), args.game) {
        error!(e, "Mount failed");
        return ExitCode::FAILURE;
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

    let modules = ModuleManager::new(&fs, &mm, 1024 * 1024);

    info!("{} modules are available.", modules.available_count());

    // Initialize syscall routines.
    info!("Initializing system call routines.");

    let syscalls = Syscalls::new();

    // Load eboot.bin.
    let mut loaded = Vec::new();

    info!("Loading eboot.bin.");

    let eboot = match modules.load_eboot() {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
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
            Err(module::LoadError::NotFound) => {
                warn!("Module {name} not found, skipping.");
                continue;
            }
            Err(e) => {
                error!(e, "Cannot load {name}");
                return ExitCode::FAILURE;
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

    info!("{} module(s) have been loaded successfully.", loaded.len());

    // Apply module relocations.
    for module in loaded {
        info!("Applying relocation entries on {}.", module.image().name());

        if let Err(e) = unsafe { module.apply_relocs(|h, n| modules.resolve_symbol(h, n)) } {
            error!(e, "Apply failed");
            return ExitCode::FAILURE;
        }
    }

    // Get execution engine.
    info!("Initializing execution engine.");

    match args.execution_engine {
        Some(ee) => match ee {
            #[cfg(target_arch = "x86_64")]
            ExecutionEngine::Native => exec_with_native(&modules, &syscalls),
            #[cfg(not(target_arch = "x86_64"))]
            ExecutionEngine::Native => {
                error!("Native execution engine cannot be used on your machine.");
                return false;
            }
            ExecutionEngine::Llvm => exec_with_llvm(&llvm, &modules),
        },
        #[cfg(target_arch = "x86_64")]
        None => exec_with_native(&modules, &syscalls),
        #[cfg(not(target_arch = "x86_64"))]
        None => exec_with_llvm(&llvm, &modules),
    }
}

#[cfg(target_arch = "x86_64")]
fn exec_with_native(modules: &ModuleManager, syscalls: &Syscalls) -> ExitCode {
    let mut ee = ee::native::NativeEngine::new(modules, syscalls);

    info!("Patching modules.");

    match unsafe { ee.patch_mods() } {
        Ok(r) => {
            let mut t = 0;

            for (m, c) in r {
                if c != 0 {
                    info!("{c} patch(es) have been applied to {m}.");
                    t += 1;
                }
            }

            info!("{t} module(s) have been patched successfully.");
        }
        Err(e) => {
            error!(e, "Patch failed");
            return ExitCode::FAILURE;
        }
    }

    exec(ee)
}

fn exec_with_llvm(llvm: &Llvm, modules: &ModuleManager) -> ExitCode {
    let mut ee = ee::llvm::LlvmEngine::new(llvm, modules);

    info!("Lifting modules.");

    if let Err(e) = ee.lift_modules() {
        error!(e, "Lift failed");
        return ExitCode::FAILURE;
    }

    exec(ee)
}

fn exec<E: ee::ExecutionEngine>(mut ee: E) -> ExitCode {
    info!("Starting application.");

    if let Err(e) = ee.run() {
        error!(e, "Start failed");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
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

    // Memory.
    let mem = module.memory();

    info!(
        "Memory address: {:#018x}:{:#018x}",
        mem.addr(),
        mem.addr() + mem.len()
    );

    if let Some(entry) = image.entry_addr() {
        info!("Entry address : {:#018x}", mem.addr() + entry);
    }

    for s in mem.segments().iter() {
        let addr = mem.addr() + s.start();

        info!(
            "Program {} is mapped to {:#018x}:{:#018x} with {}.",
            s.program(),
            addr,
            addr + s.len(),
            image.programs()[s.program()].flags(),
        );
    }
}
