use crate::arc4::Arc4;
use crate::ee::EntryArg;
use crate::fs::{Fs, VPath};
use crate::llvm::Llvm;
use crate::log::{print, LOGGER};
use crate::memory::{MappingFlags, MemoryManager, Protections};
use crate::process::VProc;
use crate::rtld::{ModuleFlags, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
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
mod process;
mod rtld;
mod signal;
mod syscalls;
mod sysctl;
mod thread;

fn main() -> ExitCode {
    log::init();

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

    // Initialize debug dump.
    if let Some(path) = &args.debug_dump {
        // Remove previous dump.
        if args.clear_debug_dump {
            if let Err(e) = remove_dir_all(path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!(e, "Failed to remove {}", path.display());
                }
            }
        }

        // Create a directory.
        if let Err(e) = create_dir_all(path) {
            warn!(e, "Failed to create {}", path.display());
        }

        // Create log file for us.
        let log = path.join("obliteration.log");

        match File::create(&log) {
            Ok(v) => LOGGER.get().unwrap().set_file(v),
            Err(e) => warn!(e, "Failed to create {}", log.display()),
        }
    }

    // Show basic infomation.
    let mut log = info!();

    writeln!(log, "Starting Obliteration Kernel.").unwrap();
    writeln!(log, "System directory    : {}", args.system.display()).unwrap();
    writeln!(log, "Game directory      : {}", args.game.display()).unwrap();

    if let Some(v) = &args.debug_dump {
        writeln!(log, "Debug dump directory: {}", v.display()).unwrap();
    }

    print(log);

    // Initialize Arc4.
    info!("Initializing arc4random.");

    let arc4: &'static Arc4 = Box::leak(Arc4::new().into());

    // Initialize LLVM.
    info!("Initializing LLVM.");
    Llvm::init();

    let sysctl = Sysctl::new(arc4);

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
    let mut log = info!();

    writeln!(log, "Page size is             : {}", mm.page_size()).unwrap();
    writeln!(
        log,
        "Allocation granularity is: {}",
        mm.allocation_granularity()
    )
    .unwrap();

    print(log);

    // Initialize virtual process.
    info!("Initializing virtual process.");

    VProc::new();

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let mut ld = match RuntimeLinker::new(&fs) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    // Print application module.
    let mut log = info!();

    writeln!(log, "Application   : {}", ld.app().path()).unwrap();
    ld.app().print(log);

    // Preload libkernel.
    let path: &VPath = "/system/common/lib/libkernel.sprx".try_into().unwrap();

    info!("Loading {path}.");

    let module = match ld.load(path, true) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
        }
    };

    module.flags_mut().remove(ModuleFlags::UNK2);
    module.print(info!());

    ld.set_kernel(module);

    // Preload libSceLibcInternal.
    let path: &VPath = "/system/common/lib/libSceLibcInternal.sprx"
        .try_into()
        .unwrap();

    info!("Loading {path}.");

    let module = match ld.load(path, true) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
        }
    };

    module.flags_mut().remove(ModuleFlags::UNK2);
    module.print(info!());

    drop(module);

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
                    let mut l = info!();
                    let mut t = 0;

                    for (m, c) in r {
                        if c != 0 {
                            writeln!(l, "{c} patch(es) have been applied to {m}.").unwrap();
                            t += 1;
                        }
                    }

                    writeln!(l, "{t} module(s) have been patched successfully.").unwrap();
                    print(l);
                }
                Err(e) => {
                    error!(e, "Patch failed");
                    return ExitCode::FAILURE;
                }
            }

            exec(ee, &ld)
        }
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!(
                logger,
                "Native execution engine cannot be used on your machine."
            );
            return ExitCode::FAILURE;
        }
        ExecutionEngine::Llvm => {
            let mut ee = ee::llvm::LlvmEngine::new(&ld);

            info!("Lifting modules.");

            if let Err(e) = ee.lift_initial_modules() {
                error!(e, "Lift failed");
                return ExitCode::FAILURE;
            }

            exec(ee, &ld)
        }
    }
}

fn exec<E: ee::ExecutionEngine>(mut ee: E, ld: &RuntimeLinker) -> ExitCode {
    // Setup entry argument.
    let arg = EntryArg::new(ld.app().path());

    // TODO: Check how the PS4 allocate the stack.
    // TODO: We should allocate a guard page to catch stack overflow.
    info!("Allocating application stack.");

    let stack = match MemoryManager::current().mmap(
        0,
        1024 * 1024 * 10, // 10MB should be large enough.
        Protections::CPU_READ | Protections::CPU_WRITE,
        MappingFlags::MAP_ANON | MappingFlags::MAP_PRIVATE,
        -1,
        0,
    ) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Allocate failed");
            return ExitCode::FAILURE;
        }
    };

    // Start the application.
    info!("Starting application.");

    if let Err(e) = unsafe { ee.run(arg, stack) } {
        error!(e, "Start failed");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[derive(Parser, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Args {
    #[arg(long)]
    system: PathBuf,

    #[arg(long)]
    game: PathBuf,

    #[arg(long)]
    debug_dump: Option<PathBuf>,

    #[arg(long)]
    #[serde(default)]
    clear_debug_dump: bool,

    #[arg(long, short)]
    execution_engine: Option<ExecutionEngine>,
}

#[derive(Clone, ValueEnum, Deserialize)]
enum ExecutionEngine {
    Native,
    Llvm,
}
