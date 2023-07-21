use crate::arc4::Arc4;
use crate::fs::path::VPath;
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::log::Logger;
use crate::memory::MemoryManager;
use crate::process::VProc;
use crate::rtld::{Module, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use crate::thread::VThread;
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::RwLock;

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
    // Initialize logger.
    let mut logger = Logger::new();

    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = match File::open(".kernel-debug") {
            Ok(v) => v,
            Err(e) => {
                error!(logger, e, "Failed to open .kernel-debug");
                return ExitCode::FAILURE;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                error!(logger, e, "Failed to read .kernel-debug");
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
                error!(logger, e, "Failed to remove {}", args.debug_dump.display());
                return ExitCode::FAILURE;
            }
        }
    }

    // Begin File logging
    let debug_dump_dir = PathBuf::from(&args.debug_dump);
    let log_file_path = debug_dump_dir.join("obliteration-kernel.log");
    if let Ok(_) = logger.set_log_file(&log_file_path) {
        info!(logger, "File Logging enabled");
    } else {
        warn!(logger, "Failed to set log file at: {:?}", log_file_path);
    }

    // Show basic infomation.
    info!(logger, "Starting Obliteration kernel.");
    info!(logger, "System directory    : {}", args.system.display());
    info!(logger, "Game directory      : {}", args.game.display());
    info!(
        logger,
        "Debug dump directory: {}",
        args.debug_dump.display()
    );

    // Initialize foundations.
    let arc4 = Arc4::new();
    let llvm = Llvm::new();
    let sysctl = Sysctl::new(&arc4);

    // Initialize filesystem.
    info!(logger, "Initializing file system.");

    let fs = match Fs::new(args.system, args.game) {
        Ok(v) => v,
        Err(e) => {
            error!(logger, e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    // Initialize memory manager.
    info!(logger, "Initializing memory manager.");

    let mm = MemoryManager::new();

    info!(logger, "Page size is             : {}", mm.page_size());
    info!(
        logger,
        "Allocation granularity is: {}",
        mm.allocation_granularity()
    );

    // Initialize virtual process.
    info!(logger, "Initializing virtual process.");

    let mut proc = VProc::new();

    proc.push_thread(VThread::new(std::thread::current().id()));

    // Initialize runtime linker.
    info!(logger, "Initializing runtime linker.");

    let mut ld = match RuntimeLinker::new(&fs, &mm) {
        Ok(v) => v,
        Err(e) => {
            error!(logger, e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    info!(
        logger,
        "Application executable: {}",
        ld.app().image().name()
    );
    print_module(&logger, ld.app());

    // Preload libkernel.
    let path: &VPath = "/system/common/lib/libkernel.sprx".try_into().unwrap();

    info!(logger, "Loading {path}.");

    let module = match ld.load(path) {
        Ok(v) => v,
        Err(e) => {
            error!(logger, e, "Load failed");
            return ExitCode::FAILURE;
        }
    };

    print_module(&logger, module);

    // Set libkernel ID.
    let id = module.id();
    ld.set_kernel(id);

    // Preload libSceLibcInternal.
    let path: &VPath = "/system/common/lib/libSceLibcInternal.sprx"
        .try_into()
        .unwrap();

    info!(logger, "Loading {path}.");

    match ld.load(path) {
        Ok(m) => print_module(&logger, m),
        Err(e) => {
            error!(logger, e, "Load failed");
            return ExitCode::FAILURE;
        }
    }

    // Initialize syscall routines.
    info!(logger, "Initializing system call routines.");

    let proc = RwLock::new(proc);
    let ld = RwLock::new(ld);
    let syscalls = Syscalls::new(&logger, &proc, &sysctl, &ld);

    // Bootstrap execution engine.
    info!(logger, "Initializing execution engine.");

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

            info!(logger, "Patching modules.");

            match unsafe { ee.patch_mods() } {
                Ok(r) => {
                    let mut t = 0;

                    for (m, c) in r {
                        if c != 0 {
                            info!(logger, "{c} patch(es) have been applied to {m}.");
                            t += 1;
                        }
                    }

                    info!(logger, "{t} module(s) have been patched successfully.");
                }
                Err(e) => {
                    error!(logger, e, "Patch failed");
                    return ExitCode::FAILURE;
                }
            }

            exec(&logger, ee)
        }
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!("Native execution engine cannot be used on your machine.");
            return ExitCode::FAILURE;
        }
        ExecutionEngine::Llvm => {
            let mut ee = ee::llvm::LlvmEngine::new(&llvm, &ld);

            info!(logger, "Lifting modules.");

            if let Err(e) = ee.lift_initial_modules() {
                error!(logger, e, "Lift failed");
                return ExitCode::FAILURE;
            }

            exec(&logger, ee)
        }
    }
}

fn exec<E: ee::ExecutionEngine>(logger: &Logger, mut ee: E) -> ExitCode {
    info!(logger, "Starting application.");

    if let Err(e) = ee.run() {
        error!(logger, e, "Start failed");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn print_module(logger: &Logger, module: &Module) {
    // Image details.
    let image = module.image();

    if image.self_segments().is_some() {
        info!(logger, "Image format  : SELF");
    } else {
        info!(logger, "Image format  : ELF");
    }

    info!(logger, "Image type    : {}", image.ty());

    if let Some(dynamic) = image.dynamic_linking() {
        let i = dynamic.module_info();

        info!(logger, "Module name   : {}", i.name());

        if let Some(f) = dynamic.flags() {
            info!(logger, "Module flags  : {f}");
        }
    }

    for (i, p) in image.programs().iter().enumerate() {
        let offset = p.offset();
        let end = offset + p.file_size();

        info!(
            logger,
            "Program {:<6}: {:#018x}:{:#018x}:{}",
            i,
            offset,
            end,
            p.ty()
        );
    }

    if let Some(dynamic) = image.dynamic_linking() {
        for n in dynamic.needed() {
            info!(logger, "Needed        : {n}");
        }
    }

    // Memory.
    let mem = module.memory();

    info!(
        logger,
        "Memory address: {:#018x}:{:#018x}",
        mem.addr(),
        mem.addr() + mem.len()
    );

    if let Some(entry) = image.entry_addr() {
        info!(logger, "Entry address : {:#018x}", mem.addr() + entry);
    }

    for s in mem.segments().iter() {
        if let Some(p) = s.program() {
            let addr = mem.addr() + s.start();

            info!(
                logger,
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
