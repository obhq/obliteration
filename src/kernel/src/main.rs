use crate::arch::MachDep;
use crate::arnd::Arnd;
use crate::budget::Budget;
use crate::ee::{EntryArg, RawFn};
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
use ee::llvm::LlvmEngine;
use ee::native::NativeEngine;
use llt::Thread;
use macros::vpath;
use serde::Deserialize;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use thistermination::Termination;

mod arch;
mod arnd;
mod budget;
mod console;
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

fn main() -> Result<(), KernelError> {
    log::init();

    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = File::open(".kernel-debug")?;

        serde_yaml::from_reader(file)?
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

    // Initialize foundations.
    let arnd = Arnd::new();
    let llvm = Llvm::new();
    let mut syscalls = Syscalls::new();
    let vp = VProc::new(&mut syscalls)?;

    // Initialize memory management.
    let mm = MemoryManager::new(&vp, &mut syscalls)?;

    let mut log = info!();

    writeln!(log, "Page size             : {:#x}", mm.page_size()).unwrap();
    writeln!(
        log,
        "Allocation granularity: {:#x}",
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

    // Select execution engine.
    match args.execution_engine.unwrap_or_default() {
        #[cfg(target_arch = "x86_64")]
        ExecutionEngine::Native => run(
            args.system,
            args.game,
            args.debug_dump,
            &arnd,
            syscalls,
            &vp,
            &mm,
            crate::ee::native::NativeEngine::new(),
        )?,
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!("Native execution engine cannot be used on your machine.");
            return ExitCode::FAILURE;
        }
        ExecutionEngine::Llvm => run(
            args.system,
            args.game,
            args.debug_dump,
            &arnd,
            syscalls,
            &vp,
            &mm,
            crate::ee::llvm::LlvmEngine::new(&llvm),
        )?,
    }

    Ok(())
}

fn run<E: crate::ee::ExecutionEngine>(
    root: PathBuf,
    app: PathBuf,
    dump: Option<PathBuf>,
    arnd: &Arc<Arnd>,
    mut syscalls: Syscalls,
    vp: &Arc<VProc>,
    mm: &Arc<MemoryManager>,
    ee: Arc<E>,
) -> Result<(), ExecutionError<E>> {
    // Initialize kernel components.
    let fs = Fs::new(root, app, vp, &mut syscalls);
    RegMgr::new(&mut syscalls);
    MachDep::new(&mut syscalls);
    Budget::new(vp, &mut syscalls);

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let ld = RuntimeLinker::new(&fs, mm, &ee, vp, &mut syscalls, dump.as_deref())?;

    // Print application module.
    let app = ld.app();
    let mut log = info!();

    writeln!(log, "Application   : {}", app.path()).unwrap();
    app.print(log);

    // Initialize sysctl.
    Sysctl::new(arnd, vp, mm, &mut syscalls);
    ee.set_syscalls(syscalls);

    // Preload libkernel.
    let path = vpath!("/system/common/lib/libkernel.sprx");

    info!("Loading {path}.");

    let module = ld.load(path, true)?;

    module.flags_mut().remove(ModuleFlags::UNK2);
    module.print(info!());

    ld.set_kernel(module);

    // Preload libSceLibcInternal.
    let path = vpath!("/system/common/lib/libSceLibcInternal.sprx");

    info!("Loading {path}.");

    let module = ld.load(path, true)?;

    module.flags_mut().remove(ModuleFlags::UNK2);
    module.print(info!());

    drop(module);

    // Get eboot.bin.
    if app.file_info().is_none() {
        todo!("statically linked eboot.bin");
    }

    // Get entry point.
    let boot = ld.kernel().unwrap();
    let mut arg = Box::pin(EntryArg::<E>::new(arnd, vp, mm, app.clone()));
    let entry = unsafe { boot.get_function(boot.entry().unwrap()) };
    let entry = move || unsafe { entry.exec1(arg.as_mut().as_vec().as_ptr()) };

    // Spawn main thread.
    info!("Starting application.");

    let stack = mm.stack();
    let runner = unsafe { vp.new_thread(stack.start(), stack.len(), entry)? };

    // Wait for main thread to exit. This should never return.
    join_thread(runner)?;

    Ok(())
}

#[cfg(unix)]
fn join_thread(thr: Thread) -> Result<(), std::io::Error> {
    let err = unsafe { libc::pthread_join(thr, std::ptr::null_mut()) };

    if err != 0 {
        Err(std::io::Error::from_raw_os_error(err))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn join_thread(thr: Thread) -> Result<(), std::io::Error> {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE};

    if unsafe { WaitForSingleObject(thr, INFINITE) } != WAIT_OBJECT_0 {
        return Err(std::io::Error::last_os_error());
    }

    assert_ne!(unsafe { CloseHandle(thr) }, 0);

    Ok(())
}

#[derive(Error, Termination)]
enum KernelError {
    #[error("Failed to open .kernel-debug -> {0}")]
    FailedToOpenKernelDebug(#[from] std::io::Error),

    #[error("Failed to parse .kernel-debug -> {0}")]
    FailedToParseKernelDebug(#[from] serde_yaml::Error),

    #[error("Cannot get CPU time limit -> {0}")]
    VirtualProcessInitialzationFailed(#[from] crate::process::VProcError),

    #[error("Cannot get CPU time limit -> {0}")]
    MemoryManagerInitializationFailed(#[from] crate::memory::MemoryManagerError),

    #[error("Execution failed -> {0}")]
    LlvmExecutionError(#[from] ExecutionError<LlvmEngine>),

    #[error("Execution failed -> {0}")]
    NativeExecutionError(#[from] ExecutionError<NativeEngine>),
}

#[derive(Debug, Error)]
enum ExecutionError<E: crate::ee::ExecutionEngine> {
    #[error("Initialize failed -> {0}")]
    InitializeFailed(#[from] crate::rtld::RuntimeLinkerError<E>),

    #[error("Load failed -> {0}")]
    LoadFailed(#[from] crate::rtld::LoadError<E>),

    #[error("Create main thread failed -> {0}")]
    SpawnError(#[from] llt::SpawnError),

    #[error("Failed join with main thread -> {0}")]
    JoinError(#[from] std::io::Error),
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

impl Default for ExecutionEngine {
    #[cfg(target_arch = "x86_64")]
    fn default() -> Self {
        ExecutionEngine::Native
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn default() -> Self {
        ExecutionEngine::Llvm
    }
}
