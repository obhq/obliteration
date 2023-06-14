use crate::fs::path::{VPath, VPathBuf};
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::memory::MemoryManager;
use crate::module::{Module, ModuleManager};
use crate::syscalls::Syscalls;
use clap::{Parser, ValueEnum};
use directories::ProjectDirs;
use log::{debug, error, info};
use serde::Deserialize;
use simplelog::*;
use std::fs::{create_dir_all, File};
use std::panic;
use std::path::PathBuf;
use std::process::ExitCode;

mod disasm;
mod ee;
mod errno;
mod fs;
mod llvm;
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
    // Create Log config.
    let logconf = ConfigBuilder::new()
        .set_location_level(LevelFilter::Error)
        .set_target_level(LevelFilter::Error)
        .set_thread_mode(ThreadLogMode::Both)
        .set_thread_level(LevelFilter::Error)
        .set_time_offset_to_local()
        .unwrap()
        .build();

    let project_dirs = ProjectDirs::from("", "OBHQ", "Obliteration").unwrap();
    let data_dir = project_dirs.data_dir();
    let log_dir = data_dir.join("log");
    create_dir_all(&log_dir).expect("Failed to create Data directory!");
    let log_path = log_dir.join("obliteration-kernel.log");

    // Start Logger
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Trace,
            logconf.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            logconf,
            File::create(log_path).unwrap(),
        ),
    ])
    .unwrap();

    // Catch Panics from the Kernel into errors.
    panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .unwrap_or("Failed to get Panic message!");

        let location = if let Some(location) = panic_info.location() {
            format!("{}:{}", location.file(), location.line())
        } else {
            "No_Location_Found".into()
        };

        error!("Panic hooked!\n[PANIC] [{}] {}", location, payload);
    }));

    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = match File::open(".kernel-debug") {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to open .kernel-debug: {}", e);
                return ExitCode::FAILURE;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to read .kernel-debug: {}", e);
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
                error!("Failed to remove {}: {}", args.debug_dump.display(), e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Show basic infomation.
    info!("Starting Obliteration kernel.");
    debug!("Debug dump directory is: {}.", args.debug_dump.display());

    // Initialize LLVM.
    let llvm = Llvm::new();

    // Initialize filesystem.
    let fs = Fs::new();

    info!("Mounting / to {}.", args.system.display());

    if let Err(e) = fs.mount(VPathBuf::new(), args.system) {
        error!("Mount failed: {}", e);
        return ExitCode::FAILURE;
    }

    info!("Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount(VPath::new("/mnt/app0").unwrap(), args.game) {
        error!("Mount failed: {}", e);
        return ExitCode::FAILURE;
    }

    // Initialize memory manager.
    info!("Initializing memory manager.");

    let mm = MemoryManager::new();

    debug!("Page size is: {}.", mm.page_size());
    debug!(
        "Allocation granularity is: {}.",
        mm.allocation_granularity()
    );

    // Initialize the module manager.
    info!("Initializing module manager.");

    let modules = ModuleManager::new(&fs, &mm, 1024 * 1024);

    // Initialize syscall routines.
    info!("Initializing system call routines.");

    let syscalls = Syscalls::new();

    // Load eboot.bin.
    info!("Loading eboot.bin.");

    match modules.load_eboot() {
        Ok(m) => print_module(&m),
        Err(e) => {
            error!("Load failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Preload libkernel.
    let libkernel: &VPath = "/system/common/lib/libkernel.sprx".try_into().unwrap();

    info!("Loading {libkernel}.");

    match modules.load_file(libkernel) {
        Ok(m) => print_module(&m),
        Err(e) => {
            error!("Load failed: {}", e);
            return ExitCode::FAILURE;
        }
    }

    // Preload internal libc.
    let libc: &VPath = "/system/common/lib/libSceLibcInternal.sprx"
        .try_into()
        .unwrap();

    info!("Loading {libc}.");

    match modules.load_file(libc) {
        Ok(m) => print_module(&m),
        Err(e) => {
            error!("Load failed: {}", e);
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
                    debug!("{c} patch(es) have been applied to {m}.");
                    t += 1;
                }
            }

            debug!("{t} module(s) have been patched successfully.");
        }
        Err(e) => {
            error!("Patch failed: {}", e);
            return ExitCode::FAILURE;
        }
    }

    exec(ee)
}

fn exec_with_llvm(llvm: &Llvm, modules: &ModuleManager) -> ExitCode {
    let mut ee = ee::llvm::LlvmEngine::new(llvm, modules);

    info!("Lifting modules.");

    if let Err(e) = ee.lift_modules() {
        error!("Lift failed: {}", e);
        return ExitCode::FAILURE;
    }

    exec(ee)
}

fn exec<E: ee::ExecutionEngine>(mut ee: E) -> ExitCode {
    info!("Starting application.");

    if let Err(e) = ee.run() {
        error!("Start failed: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn print_module(module: &Module) {
    // Image type.
    let image = module.image();

    if image.self_segments().is_some() {
        debug!("Image type    : SELF");
    } else {
        debug!("Image type    : ELF");
    }

    // Dynamic linking.
    if let Some(dynamic) = image.dynamic_linking() {
        let i = dynamic.module_info();

        debug!("Module name   : {}", i.name());

        if let Some(f) = dynamic.flags() {
            debug!("Module flags  : {f}");
        }
    }

    // Memory.
    let mem = module.memory();

    debug!(
        "Memory address: {:#018x}:{:#018x}",
        mem.addr(),
        mem.addr() + mem.len()
    );

    if let Some(entry) = image.entry_addr() {
        debug!("Entry address : {:#018x}", mem.addr() + entry);
    }

    for s in mem.segments().iter() {
        let addr = mem.addr() + s.start();

        debug!(
            "Program {} is mapped to {:#018x}:{:#018x} with {}.",
            s.program(),
            addr,
            addr + s.len(),
            image.programs()[s.program()].flags(),
        );
    }
}
