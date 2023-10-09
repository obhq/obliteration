use crate::arnd::Arnd;
use crate::ee::EntryArg;
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::log::{print, LOGGER};
use crate::memory::MemoryManager;
use crate::process::VProc;
use crate::regmgr::RegMgr;
use crate::rtld::{ModuleFlags, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use clap::{Parser, ValueEnum};
use macros::vpath;
use serde::Deserialize;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

mod arnd;
mod console;
mod disasm;
mod ee;
mod errno;
mod fs;
mod idt;
mod llvm;
mod log;
mod memory;
mod process;
mod regmgr;
mod rtld;
mod signal;
mod syscalls;
mod sysctl;
mod ucred;

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

    // Initialize base systems.
    let arnd: &'static Arnd = Box::leak(Arnd::new().into());
    let llvm: &'static Llvm = Box::leak(Llvm::new().into());
    let regmgr: &'static RegMgr = Box::leak(RegMgr::new().into());
    let fs: &'static Fs = Box::leak(Fs::new(args.system, args.game).into());
    let vp: &'static VProc = match VProc::new() {
        Ok(v) => Box::leak(v.into()),
        Err(e) => {
            error!(e, "Virtual process initialization failed");
            return ExitCode::FAILURE;
        }
    };

    // Initialize memory management.
    let mm: &'static MemoryManager = match MemoryManager::new(vp) {
        Ok(v) => Box::leak(v.into()),
        Err(e) => {
            error!(e, "Memory manager initialization failed");
            return ExitCode::FAILURE;
        }
    };

    let mut log = info!();

    writeln!(log, "Page size             : {}", mm.page_size()).unwrap();
    writeln!(
        log,
        "Allocation granularity: {}",
        mm.allocation_granularity()
    )
    .unwrap();
    writeln!(
        log,
        "Main stack            : {:p}:{:p}",
        mm.stack().start(),
        mm.stack().end()
    )
    .unwrap();

    print(log);

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let ld: &'static mut RuntimeLinker = match RuntimeLinker::new(fs, mm, vp) {
        Ok(v) => Box::leak(v.into()),
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
    let path = vpath!("/system/common/lib/libkernel.sprx");

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
    let path = vpath!("/system/common/lib/libSceLibcInternal.sprx");

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

    // Bootstrap execution engine.
    let sysctl: &'static Sysctl = Box::leak(Sysctl::new(arnd, vp, mm).into());
    let syscalls: &'static Syscalls =
        Box::leak(Syscalls::new(vp, fs, mm, ld, sysctl, regmgr).into());
    let arg = EntryArg::new(arnd, vp, mm, ld.app().clone());
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
            let mut ee = ee::native::NativeEngine::new(vp, mm, ld, syscalls);

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

            exec(ee, arg)
        }
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!("Native execution engine cannot be used on your machine.");
            return ExitCode::FAILURE;
        }
        ExecutionEngine::Llvm => {
            let mut ee = ee::llvm::LlvmEngine::new(llvm, ld);

            info!("Lifting modules.");

            if let Err(e) = ee.lift_initial_modules() {
                error!(e, "Lift failed");
                return ExitCode::FAILURE;
            }

            exec(ee, arg)
        }
    }
}

fn exec<E: ee::ExecutionEngine>(mut ee: E, arg: EntryArg) -> ExitCode {
    // Start the application.
    info!("Starting application.");

    if let Err(e) = unsafe { ee.run(arg) } {
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
