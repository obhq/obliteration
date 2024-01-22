use crate::arch::MachDep;

use crate::arnd::Arnd;
use crate::budget::{Budget, BudgetManager, ProcType};
use crate::dmem::DmemManager;
use crate::ee::{EntryArg, RawFn};
use crate::fs::Fs;
use crate::llvm::Llvm;
use crate::log::{print, LOGGER};
use crate::memory::MemoryManager;
use crate::process::{VProc, VThread};
use crate::regmgr::RegMgr;
use crate::rtld::{LoadFlags, ModuleFlags, RuntimeLinker};
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use crate::tty::TtyManager;
use crate::ucred::{AuthAttrs, AuthCaps, AuthInfo, AuthPaid, Gid, Ucred, Uid};
use clap::{Parser, ValueEnum};
use fs::FsError;
use llt::{OsThread, SpawnError};
use macros::vpath;
use memory::MemoryManagerError;
use param::Param;
use process::VProcInitError;
use serde::Deserialize;
use std::error::Error;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{ExitCode, Termination};
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::{MemoryRefreshKind, System};
use thiserror::Error;
use tty::TtyInitError;

mod arch;
mod arnd;
mod budget;
mod dmem;
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
mod tty;
mod ucred;

fn main() -> Exit {
    start().into()
}

fn start() -> Result<(), KernelError> {
    // Begin logger.
    log::init();

    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = File::open(".kernel-debug").map_err(KernelError::FailedToOpenDebugConfig)?;

        serde_yaml::from_reader(file).map_err(KernelError::FailedToParseDebugConfig)?
    } else {
        Args::try_parse()?
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
    let param = File::open(&path).map_err(KernelError::FailedToOpenGameParam)?;

    // Load param.sfo.
    let param = Arc::new(Param::read(param)?);

    // Get auth info for the process.
    let auth =
        AuthInfo::from_title_id(param.title_id()).ok_or(KernelError::InvalidTitleId(path))?;

    // Show basic information.
    let mut log = info!();
    let mut hwinfo = System::new_with_specifics(
        sysinfo::RefreshKind::new()
            .with_memory(sysinfo::MemoryRefreshKind::new())
            .with_cpu(sysinfo::CpuRefreshKind::new()),
    );
    hwinfo.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());

    // Init information
    writeln!(log, "Starting Obliteration Kernel.").unwrap();
    writeln!(log, "System directory    : {}", args.system.display()).unwrap();
    writeln!(log, "Game directory      : {}", args.game.display()).unwrap();

    if let Some(v) = &args.debug_dump {
        writeln!(log, "Debug dump directory: {}", v.display()).unwrap();
    }

    // Param information
    writeln!(log, "Application Title   : {}", param.title().unwrap()).unwrap();
    writeln!(log, "Application ID      : {}", param.title_id()).unwrap();
    writeln!(log, "Application Category: {}", param.category()).unwrap();
    writeln!(log, "Application Version : {}", param.app_ver().unwrap()).unwrap();

    // Hardware information
    writeln!(
        log,
        "Operating System    : {} {}",
        System::long_os_version().unwrap_or_else(|| "Unknown OS".to_string()),
        if cfg!(target_os = "windows") {
            System::kernel_version().unwrap_or_else(|| "Unknown Kernel".to_string())
        } else {
            "".to_string()
        }
    )
    .unwrap();
    writeln!(log, "CPU Information     : {}", hwinfo.cpus()[0].brand()).unwrap();
    writeln!(
        log,
        "Memory Available    : {}/{} MB",
        hwinfo.available_memory() / 1048576,
        hwinfo.total_memory() / 1048576
    )
    .unwrap();
    writeln!(log, "Pro mode            : {}", args.pro).unwrap();

    print(log);

    // Setup kernel credential.
    let cred = Arc::new(Ucred::new(
        Uid::ROOT,
        Uid::ROOT,
        vec![Gid::ROOT],
        AuthInfo {
            paid: AuthPaid::KERNEL,
            caps: AuthCaps::new([0x4000000000000000, 0, 0, 0]),
            attrs: AuthAttrs::new([0, 0, 0, 0]),
            unk: [0; 64],
        },
    ));

    // Initialize foundations.
    let arnd = Arnd::new();
    let llvm = Llvm::new();
    let mut syscalls = Syscalls::new();

    // Initializes filesystem.
    let fs = Fs::new(args.system, args.game, &param, &cred, &mut syscalls)?;

    // Initialize memory management.
    let mm = MemoryManager::new(&mut syscalls)?;

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
            args.debug_dump,
            &param,
            auth,
            &arnd,
            syscalls,
            &fs,
            &mm,
            crate::ee::native::NativeEngine::new(),
        ),
        #[cfg(not(target_arch = "x86_64"))]
        ExecutionEngine::Native => {
            error!("Native execution engine cannot be used on your machine.");
            Err(KernelError::NativeExecutionEngineNotSupported)
        }
        ExecutionEngine::Llvm => run(
            args.debug_dump,
            &param,
            auth,
            &arnd,
            syscalls,
            &fs,
            &mm,
            crate::ee::llvm::LlvmEngine::new(&llvm),
        ),
    }
}

fn run<E: crate::ee::ExecutionEngine>(
    dump: Option<PathBuf>,
    param: &Arc<Param>,
    auth: AuthInfo,
    arnd: &Arc<Arnd>,
    mut syscalls: Syscalls,
    fs: &Arc<Fs>,
    mm: &Arc<MemoryManager>,
    ee: Arc<E>,
) -> Result<(), KernelError> {
    // Initialize TTY system.
    let _tty = TtyManager::new()?;

    // Initialize kernel components.
    RegMgr::new(&mut syscalls);
    let machdep = MachDep::new(&mut syscalls);
    let budget = BudgetManager::new(&mut syscalls);
    DmemManager::new(&fs, &mut syscalls);
    Sysctl::new(arnd, mm, &machdep, &mut syscalls);

    // TODO: Get correct budget name from the PS4.
    let budget_id = budget.create(Budget::new("big app", ProcType::BigApp));
    let proc = VProc::new(
        auth,
        budget_id,
        ProcType::BigApp,
        1,         // See sys_budget_set on the PS4.
        fs.root(), // TODO: Change to a proper value once FS rework is done.
        "QXuNNl0Zhn",
        &mut syscalls,
    )?;

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let ld = RuntimeLinker::new(&fs, mm, &ee, &mut syscalls, dump.as_deref())
        .map_err(|e| KernelError::RuntimeLinkerInitFailed(e.into()))?;

    ee.set_syscalls(syscalls);

    // Print application module.
    let app = ld.app();
    let mut log = info!();

    writeln!(log, "Application   : {}", app.path()).unwrap();
    app.print(log);

    // Preload libkernel.
    let mut flags = LoadFlags::UNK1;
    let path = vpath!("/system/common/lib/libkernel.sprx");

    if proc.budget_ptype() == ProcType::BigApp {
        flags |= LoadFlags::BIG_APP;
    }

    info!("Loading {path}.");

    let libkernel = ld
        .load(&proc, path, flags, false, true)
        .map_err(|e| KernelError::FailedToLoadLibkernel(e.into()))?;

    libkernel.flags_mut().remove(ModuleFlags::UNK2);
    libkernel.print(info!());

    ld.set_kernel(libkernel);

    // Preload libSceLibcInternal.
    let path = vpath!("/system/common/lib/libSceLibcInternal.sprx");

    info!("Loading {path}.");

    let libc = ld
        .load(&proc, path, flags, false, true)
        .map_err(|e| KernelError::FailedToLoadLibSceLibcInternal(e.into()))?;

    libc.flags_mut().remove(ModuleFlags::UNK2);
    libc.print(info!());

    drop(libc);

    // Get eboot.bin.
    if app.file_info().is_none() {
        todo!("statically linked eboot.bin");
    }

    // Get entry point.
    let boot = ld.kernel().unwrap();
    let mut arg = Box::pin(EntryArg::<E>::new(arnd, &proc, mm, app.clone()));
    let entry = unsafe { boot.get_function(boot.entry().unwrap()) };
    let entry = move || unsafe { entry.exec1(arg.as_mut().as_vec().as_ptr()) };

    // Spawn main thread.
    info!("Starting application.");

    // TODO: Check how this constructed.
    let cred = Arc::new(Ucred::new(
        Uid::ROOT,
        Uid::ROOT,
        vec![Gid::ROOT],
        AuthInfo::SYS_CORE.clone(),
    ));

    let main = VThread::new(proc, &cred);
    let stack = mm.stack();
    let main: OsThread = unsafe { main.start(stack.start(), stack.len(), entry) }?;

    // Begin Discord Rich Presence before blocking current thread.
    if let Err(e) = discord_presence(param) {
        warn!(e, "Failed to setup Discord rich presence");
    }

    // Wait for main thread to exit. This should never return.
    join_thread(main).map_err(KernelError::FailedToJoinMainThread)?;

    Ok(())
}

fn discord_presence(param: &Param) -> Result<(), DiscordPresenceError> {
    use discord_rich_presence::activity::{Activity, Assets, Timestamps};
    use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

    // Initialize new Discord IPC with our ID.
    info!("Initializing Discord rich presence.");

    let mut client = DiscordIpcClient::new("1168617561244565584")
        .map_err(DiscordPresenceError::FailedToCreateIpc)?;

    // Attempt to have IPC connect to user's Discord, will fail if user doesn't have Discord running.
    if client.connect().is_err() {
        // No Discord running should not be a warning.
        return Ok(());
    }

    // Create details about game.
    let details = format!("Playing {} - {}", param.title().unwrap(), param.title_id());
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

    client
        .set_activity(payload)
        .map_err(DiscordPresenceError::FailedToUpdatePresence)?;

    // Keep client alive forever.
    Box::leak(client.into());

    Ok(())
}

#[cfg(unix)]
fn join_thread(thr: OsThread) -> Result<(), std::io::Error> {
    let err = unsafe { libc::pthread_join(thr, std::ptr::null_mut()) };

    if err != 0 {
        Err(std::io::Error::from_raw_os_error(err))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn join_thread(thr: OsThread) -> Result<(), std::io::Error> {
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

    #[arg(long)]
    #[serde(default)]
    pro: bool,

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

#[derive(Debug, Error)]
enum DiscordPresenceError {
    #[error("failed to create Discord IPC")]
    FailedToCreateIpc(#[source] Box<dyn Error>),

    #[error("failed to update Discord presence")]
    FailedToUpdatePresence(#[source] Box<dyn Error>),
}

#[derive(Debug, Error)]
enum KernelError {
    #[error("couldn't open .kernel-debug")]
    FailedToOpenDebugConfig(#[source] std::io::Error),

    #[error("couldn't parse .kernel-debug")]
    FailedToParseDebugConfig(#[source] serde_yaml::Error),

    #[error("couldn't parse arguments")]
    FailedToParseArgs(#[from] clap::Error),

    #[error("couldn't open param.sfo")]
    FailedToOpenGameParam(#[source] std::io::Error),

    #[error("couldn't read param.sfo ")]
    FailedToReadGameParam(#[from] param::ReadError),

    #[error("{0} has an invalid title identifier")]
    InvalidTitleId(PathBuf),

    #[error("filesystem initialization failed")]
    FilesystemInitFailed(#[from] FsError),

    #[error("memory manager initialization failed")]
    MemoryManagerInitFailed(#[from] MemoryManagerError),

    #[cfg(not(target_arch = "x86_64"))]
    #[error("the native execution engine is only supported on x86_64")]
    NativeExecutionEngineNotSupported,

    #[error("tty initialization failed")]
    TtyInitFailed(#[from] TtyInitError),

    #[error("virtual process initialization failed")]
    VProcInitFailed(#[from] VProcInitError),

    #[error("runtime linker initialization failed")]
    RuntimeLinkerInitFailed(#[source] Box<dyn Error>),

    #[error("libkernel couldn't be loaded")]
    FailedToLoadLibkernel(#[source] Box<dyn Error>),

    #[error("libSceLibcInternal couldn't be loaded")]
    FailedToLoadLibSceLibcInternal(#[source] Box<dyn Error>),

    #[error("main thread couldn't be created")]
    FailedToCreateMainThread(#[from] SpawnError),

    #[error("failed to join with main thread")]
    FailedToJoinMainThread(#[source] std::io::Error),
}

/// We have to use this for a custom implementation of the [`Termination`] trait, because
/// we need to log the error using our own error! macro instead of [`std::fmt::Debug::fmt`],
/// which is what the default implementation of Termination uses for [`Result<T: Termination, E: Debug>`].
enum Exit {
    Ok,
    Err(KernelError),
}

impl Termination for Exit {
    fn report(self) -> ExitCode {
        match self {
            Exit::Ok => ExitCode::SUCCESS,
            Exit::Err(e) => {
                error!(e, "Error while running kernel");
                ExitCode::FAILURE
            }
        }
    }
}

impl From<Result<(), KernelError>> for Exit {
    fn from(r: Result<(), KernelError>) -> Self {
        match r {
            Ok(_) => Exit::Ok,
            Err(e) => Exit::Err(e),
        }
    }
}
