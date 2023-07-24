use crate::arc4::Arc4;
use crate::fs::path::VPath;
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::log::{print, LogEntry, LogMeta, Logger, LOGGER};
use crate::memory::MemoryManager;
use crate::process::VProc;
use crate::rtld::{Module, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use crate::thread::VThread;
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::RwLock;
use termcolor::{Color, ColorSpec};

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
    LOGGER.set(Logger::new()).unwrap();

    std::panic::set_hook(Box::new(|i| {
        if let Some(l) = LOGGER.get() {
            // Setup meta.
            let mut m = LogMeta {
                category: 'P',
                color: ColorSpec::new(),
                file: i.location().map(|l| l.file()),
                line: i.location().map(|l| l.line()),
            };

            m.color.set_fg(Some(Color::Magenta)).set_bold(true);

            // Write.
            let mut e = l.entry(m);

            if let Some(&p) = i.payload().downcast_ref::<&str>() {
                writeln!(e, "{p}").unwrap();
            } else if let Some(p) = i.payload().downcast_ref::<String>() {
                writeln!(e, "{p}").unwrap();
            } else {
                writeln!(e, "Don't know how to print the panic payload.").unwrap();
            }

            l.write(e);
        } else {
            println!("{i}");
        }
    }));

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
        if args.clear_debug_dump.unwrap_or(false) {
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

    let mut proc = VProc::new();

    proc.push_thread(VThread::new(std::thread::current().id()));

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let mut ld = match RuntimeLinker::new(&fs, &mm) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    // Print application module.
    let mut log = info!();

    writeln!(log, "Application   : {}", ld.app().image().name()).unwrap();
    print_module(&mut log, ld.app());

    print(log);

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

    // Print libkernel.
    let mut log = info!();

    print_module(&mut log, module);
    print(log);

    // Set libkernel ID.
    let id = module.id();
    ld.set_kernel(id);

    // Preload libSceLibcInternal.
    let path: &VPath = "/system/common/lib/libSceLibcInternal.sprx"
        .try_into()
        .unwrap();

    info!("Loading {path}.");

    match ld.load(path) {
        Ok(m) => {
            let mut l = info!();
            print_module(&mut l, m);
            print(l);
        }
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
        }
    }

    // Initialize syscall routines.
    info!("Initializing system call routines.");

    let proc = RwLock::new(proc);
    let ld = RwLock::new(ld);
    let syscalls = Syscalls::new(&proc, &sysctl, &ld);

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

            exec(ee)
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
            let mut ee = ee::llvm::LlvmEngine::new(&llvm, &ld);

            info!("Lifting modules.");

            if let Err(e) = ee.lift_initial_modules() {
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

fn print_module(log: &mut LogEntry, module: &Module) {
    // Image details.
    let image = module.image();

    if image.self_segments().is_some() {
        writeln!(log, "Image format  : SELF").unwrap();
    } else {
        writeln!(log, "Image format  : ELF").unwrap();
    }

    writeln!(log, "Image type    : {}", image.ty()).unwrap();

    if let Some(dynamic) = image.dynamic_linking() {
        let i = dynamic.module_info();

        writeln!(log, "Module name   : {}", i.name()).unwrap();

        if let Some(f) = dynamic.flags() {
            writeln!(log, "Module flags  : {f}").unwrap();
        }
    }

    for (i, p) in image.programs().iter().enumerate() {
        let offset = p.offset();
        let end = offset + p.file_size();

        writeln!(
            log,
            "Program {:<6}: {:#018x}:{:#018x}:{}",
            i,
            offset,
            end,
            p.ty()
        )
        .unwrap();
    }

    if let Some(dynamic) = image.dynamic_linking() {
        for n in dynamic.needed() {
            writeln!(log, "Needed        : {n}").unwrap();
        }
    }

    // Runtime info.
    writeln!(log, "TLS index     : {}", module.tls_index()).unwrap();

    // Memory.
    let mem = module.memory();

    writeln!(
        log,
        "Memory address: {:#018x}:{:#018x}",
        mem.addr(),
        mem.addr() + mem.len()
    )
    .unwrap();

    if let Some(entry) = image.entry_addr() {
        writeln!(log, "Entry address : {:#018x}", mem.addr() + entry).unwrap();
    }

    for s in mem.segments().iter() {
        if let Some(p) = s.program() {
            let addr = mem.addr() + s.start();

            writeln!(
                log,
                "Program {} is mapped to {:#018x}:{:#018x} with {}.",
                p,
                addr,
                addr + s.len(),
                image.programs()[p].flags(),
            )
            .unwrap();
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
    debug_dump: Option<PathBuf>,

    #[arg(long)]
    clear_debug_dump: Option<bool>,

    #[arg(long, short)]
    execution_engine: Option<ExecutionEngine>,
}

#[derive(Clone, ValueEnum, Deserialize)]
enum ExecutionEngine {
    Native,
    Llvm,
}
