use crate::arch::MachDep;
use crate::arnd::Arnd;
use crate::budget::{Budget, BudgetManager, ProcType};
use crate::ee::{EntryArg, RawFn};
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::log::{print, LOGGER};
use crate::memory::MemoryManager;
use crate::process::VProc;
use crate::regmgr::RegMgr;
use crate::rtld::{LoadFlags, ModuleFlags, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use clap::{Parser, ValueEnum};
use llt::Thread;
use macros::vpath;
use param::Param;
use serde::Deserialize;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::{CpuExt, System, SystemExt};

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

fn main() -> ExitCode {
    // Begin logger
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

    // Get path to param.sfo.
    let mut path = args.game.join("sce_sys");

    path.push("param.sfo");

    // Open param.sfo.
    let param = match File::open(&path) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Cannot open {}", path.display());
            return ExitCode::FAILURE;
        }
    };

    // Load param.sfo.
    let param = match Param::read(param) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Cannot read {}", path.display());
            return ExitCode::FAILURE;
        }
    };

    // Show basic infomation.
    let mut log = info!();
    let hwinfo = System::new_with_specifics(
        sysinfo::RefreshKind::new()
            .with_memory()
            .with_cpu(sysinfo::CpuRefreshKind::new()),
    );

    // Init information
    writeln!(log, "Starting Obliteration Kernel.").unwrap();
    writeln!(log, "System directory    : {}", args.system.display()).unwrap();
    writeln!(log, "Game directory      : {}", args.game.display()).unwrap();

    if let Some(v) = &args.debug_dump {
        writeln!(log, "Debug dump directory: {}", v.display()).unwrap();
    }

    // Param information
    writeln!(log, "Application Title   : {}", param.title()).unwrap();
    writeln!(log, "Application ID      : {}", param.title_id()).unwrap();

    // Hardware information
    writeln!(
        log,
        "Operating System    : {} {}",
        hwinfo
            .long_os_version()
            .unwrap_or_else(|| "Unknown OS".to_string()),
        hwinfo
            .kernel_version()
            .unwrap_or_else(|| "Unknown Kernel".to_string())
    )
    .unwrap();
    writeln!(log, "CPU Information     : {}", hwinfo.cpus()[0].brand()).unwrap();
    writeln!(
        log,
        "Memory Available    : {}/{} MB",
        hwinfo.available_memory() / 1048576,
        hwinfo.total_memory() / 1048576
    )
    .unwrap(); // Convert Bytes to MB

    print(log);

    // Initialize foundations.
    let arnd = Arnd::new();
    let llvm = Llvm::new();
    let mut syscalls = Syscalls::new();
    let vp = match VProc::new(&mut syscalls) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Virtual process initialization failed");
            return ExitCode::FAILURE;
        }
    };

    // Initialize memory management.
    let mm = match MemoryManager::new(&vp, &mut syscalls) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Memory manager initialization failed");
            return ExitCode::FAILURE;
        }
    };

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
            &param,
            &arnd,
            syscalls,
            &vp,
            &mm,
            crate::ee::native::NativeEngine::new(),
        ),
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!("Native execution engine cannot be used on your machine.");
            return ExitCode::FAILURE;
        }
        ExecutionEngine::Llvm => run(
            args.system,
            args.game,
            args.debug_dump,
            &param,
            &arnd,
            syscalls,
            &vp,
            &mm,
            crate::ee::llvm::LlvmEngine::new(&llvm),
        ),
    }
}

fn run<E: crate::ee::ExecutionEngine>(
    root: PathBuf,
    app: PathBuf,
    dump: Option<PathBuf>,
    param: &Param,
    arnd: &Arc<Arnd>,
    mut syscalls: Syscalls,
    vp: &Arc<VProc>,
    mm: &Arc<MemoryManager>,
    ee: Arc<E>,
) -> ExitCode {
    // Initialize kernel components.
    let fs = Fs::new(root, app, param, vp, &mut syscalls);
    RegMgr::new(&mut syscalls);
    MachDep::new(&mut syscalls);
    let budget = BudgetManager::new(vp, &mut syscalls);

    // TODO: Get correct name from the PS4.
    *vp.budget_mut() = Some((
        budget.create(Budget::new("big app", ProcType::BigApp)),
        ProcType::BigApp,
    ));

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let ld = match RuntimeLinker::new(&fs, mm, &ee, vp, &mut syscalls, dump.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Initialize failed");
            return ExitCode::FAILURE;
        }
    };

    // Print application module.
    let app = ld.app();
    let mut log = info!();

    writeln!(log, "Application   : {}", app.path()).unwrap();
    app.print(log);

    // Initialize sysctl.
    Sysctl::new(arnd, vp, mm, &mut syscalls);
    ee.set_syscalls(syscalls);

    // Preload libkernel.
    let mut flags = LoadFlags::UNK1;
    let path = vpath!("/system/common/lib/libkernel.sprx");

    if vp.budget().filter(|v| v.1 == ProcType::BigApp).is_some() {
        flags |= LoadFlags::BIG_APP;
    }

    info!("Loading {path}.");

    let module = match ld.load(path, flags, false, true) {
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

    let module = match ld.load(path, flags, false, true) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return ExitCode::FAILURE;
        }
    };

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
    let runner = match unsafe { vp.new_thread(stack.start(), stack.len(), entry) } {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Create main thread failed");
            return ExitCode::FAILURE;
        }
    };

    // Begin Discord Rich Presence before blocking current thread.
    discord_presence(param);

    // Wait for main thread to exit. This should never return.
    if let Err(e) = join_thread(runner) {
        error!(e, "Failed join with main thread");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn discord_presence(param: &Param) {
    use discord_rich_presence::activity::{Activity, Assets, Timestamps};
    use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

    // Initialize new Discord IPC with our ID.
    info!("Initializing Discord rich presence.");

    let mut client = match DiscordIpcClient::new("1168617561244565584") {
        Ok(v) => v,
        Err(e) => {
            warn!(e, "Failed to create Discord IPC");
            return;
        }
    };

    // Attempt to have IPC connect to user's Discord, will fail if user doesn't have Discord running.
    if client.connect().is_err() {
        // No Discord running should not be a warning.
        return;
    }

    // Create details about game.
    let details = format!("Playing {} - {}", param.title(), param.title_id());
    let start = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Send activity to Discord.
    let payload = Activity::new()
        .details(&details)
        .assets(
            Assets::new()
                .large_image("obliteration-icon")
                .large_text("Obliteration"),
        )
        .timestamps(Timestamps::new().start(start.try_into().unwrap()));

    if let Err(e) = client.set_activity(payload) {
        // If failing here, user's Discord most likely crashed or is offline.
        warn!(e, "Failed to update Discord presence");
        return;
    }

    // Keep client alive forever.
    Box::leak(client.into());
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
