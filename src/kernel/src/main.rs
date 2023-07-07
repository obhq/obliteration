use crate::arc4::Arc4;
use crate::fs::path::VPath;
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::memory::MemoryManager;
use crate::rtld::{Module, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;

mod arc4;
mod disasm;
mod ee;
mod errno;
mod fs;
mod llvm;
mod log;
mod memory;
mod rtld;
mod syscalls;
mod sysctl;

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
    info!("System directory    : {}", args.system.display());
    info!("Game directory      : {}", args.game.display());
    info!("Debug dump directory: {}", args.debug_dump.display());

    // Initialize foundations.
    let arc4 = Arc4::new();
    let llvm = Llvm::new();
    let sysctl = Sysctl::new(&arc4);

    // Initialize filesystem.
    info!("Initializing file system.");

    let fs = match Fs::new(args.system, args.game) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    // Initialize memory manager.
    info!("Initializing memory manager.");

    let mm = MemoryManager::new();

    info!("Page size is             : {}", mm.page_size());
    info!("Allocation granularity is: {}", mm.allocation_granularity());

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let mut ld = match RuntimeLinker::new(&fs, &mm) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    info!("Application executable: {}", ld.app().image().name());
    print_module(ld.app());

    // Preload libkernel.
    let path: &VPath = "/system/common/lib/libkernel.sprx".try_into().unwrap();

    info!("Loading {path}.");

    let module = match ld.load(path) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
        }
    };

    print_module(&module);
    ld.set_kernel(module);

    // Preload libSceLibcInternal.
    let path: &VPath = "/system/common/lib/libSceLibcInternal.sprx"
        .try_into()
        .unwrap();

    info!("Loading {path}.");

    match ld.load(path) {
        Ok(m) => print_module(&m),
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
        }
    }

    // Initialize syscall routines.
    info!("Initializing system call routines.");

    let syscalls = Syscalls::new(&sysctl, &ld);

    // Bootstrap execution engine.
    info!("Initializing execution engine.");

    let ee = match args.execution_engine {
        Some(v) => v,
        #[cfg(target_arch = "x86_64")]
        None => ExecutionEngine::Native,
        #[cfg(not(target_arch = "x86_64"))]
        None => ExecutionEngine::Llvm,
    };

    match ee {
        #[cfg(target_arch = "x86_64")]
        ExecutionEngine::Native => {
            let mut ee = ee::native::NativeEngine::new(&ld, &syscalls);

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
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!("Native execution engine cannot be used on your machine.");
            return ExitCode::FAILURE;
        }
        ExecutionEngine::Llvm => {
            let mut ee = ee::llvm::LlvmEngine::new(&llvm, &ld);

            info!("Lifting modules.");

            if let Err(e) = ee.lift_modules() {
                error!(e, "Lift failed");
                return ExitCode::FAILURE;
            }

            exec(ee)
        }
    }
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
    // Image details.
    let image = module.image();

    if image.self_segments().is_some() {
        info!("Image format  : SELF");
    } else {
        info!("Image format  : ELF");
    }

    info!("Image type    : {}", image.ty());

    if let Some(dynamic) = image.dynamic_linking() {
        let i = dynamic.module_info();

        info!("Module name   : {}", i.name());

        if let Some(f) = dynamic.flags() {
            info!("Module flags  : {f}");
        }
    }

    for (i, p) in image.programs().iter().enumerate() {
        let offset = p.offset();
        let end = offset + p.file_size();

        info!(
            "Program {:<6}: {:#018x}:{:#018x}:{}",
            i,
            offset,
            end,
            p.ty()
        );
    }

    if let Some(dynamic) = image.dynamic_linking() {
        for n in dynamic.needed() {
            info!("Needed        : {n}");
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
        if let Some(p) = s.program() {
            let addr = mem.addr() + s.start();

            info!(
                "Program {} is mapped to {:#018x}:{:#018x} with {}.",
                p,
                addr,
                addr + s.len(),
                image.programs()[p].flags(),
            );
        }
    }
}

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
