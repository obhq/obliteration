use crate::arch::MachDep;
use crate::budget::{Budget, BudgetManager, ProcType};
use crate::dev::{DebugManager, TtyManager};
use crate::dmem::DmemManager;
use crate::ee::native::NativeEngine;
use crate::ee::EntryArg;
use crate::errno::EEXIST;
use crate::fs::{Fs, FsInitError, MkdirError, MountError, MountFlags, MountOpts, VPath, VPathBuf};
use crate::hv::Hypervisor;
use crate::kqueue::KernelQueueManager;
use crate::log::{print, LOGGER};
use crate::namedobj::NamedObjManager;
use crate::net::NetManager;
use crate::osem::OsemManager;
use crate::process::{VProc, VProcInitError, VThread};
use crate::regmgr::RegMgr;
use crate::rtld::{ExecError, LoadFlags, ModuleFlags, RuntimeLinker};
use crate::shm::SharedMemoryManager;
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use crate::time::TimeManager;
use crate::ucred::{AuthAttrs, AuthCaps, AuthInfo, AuthPaid, Gid, Ucred, Uid};
use crate::umtx::UmtxManager;
use clap::Parser;
use dev::{DebugManagerInitError, TtyInitError};
use dmem::DmemManagerInitError;
use llt::{OsThread, SpawnError};
use macros::vpath;
use param::Param;
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

mod arch;
mod arnd;
mod budget;
mod dev;
mod dmem;
mod ee;
mod errno;
mod fs;
mod hv;
mod idt;
mod kqueue;
mod log;
mod namedobj;
mod net;
mod osem;
mod process;
mod regmgr;
mod rtld;
mod shm;
mod signal;
mod syscalls;
mod sysctl;
mod sysent;
mod time;
mod ucred;
mod umtx;
mod vm;

fn main() -> Exit {
    run().into()
}

fn run() -> Result<(), KernelError> {
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
    writeln!(
        log,
        "Application Version : {}",
        param.app_ver().unwrap_or("UNKNOWN")
    )
    .unwrap();

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
    let hv = Hypervisor::new()?;
    let mut syscalls = Syscalls::new();
    let fs = Fs::new(args.system, &cred, &mut syscalls)?;

    // TODO: Check permission of /mnt on the PS4.
    let path = vpath!("/mnt");

    if let Err(e) = fs.mkdir(path, 0o555, None) {
        match e {
            MkdirError::CreateFailed(e) if e.errno() == EEXIST => {}
            e => return Err(KernelError::CreateDirectoryFailed(path.into(), e)),
        }
    }

    // TODO: Get mount options from the PS4.
    let mut opts = MountOpts::new();

    opts.insert("fstype", "tmpfs");
    opts.insert("fspath", path.to_owned());

    if let Err(e) = fs.mount(opts, MountFlags::empty(), None) {
        return Err(KernelError::MountFailed(path.into(), e));
    }

    // TODO: Check permission of these paths on the PS4.
    let paths = [vpath!("/mnt/sandbox"), vpath!("/mnt/sandbox/pfsmnt")];

    for path in paths {
        if let Err(e) = fs.mkdir(path, 0o555, None) {
            return Err(KernelError::CreateDirectoryFailed(path.into(), e));
        }
    }

    // TODO: Check permission of /mnt/sandbox/pfsmnt/CUSAXXXXX-app0 on the PS4.
    let game: VPathBuf = format!("/mnt/sandbox/pfsmnt/{}-app0", param.title_id())
        .try_into()
        .unwrap();

    if let Err(e) = fs.mkdir(&game, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(game, e));
    }

    // TODO: Get mount options from the PS4.
    let mut opts = MountOpts::new();

    opts.insert("fstype", "pfs");
    opts.insert("fspath", game.clone());
    opts.insert("from", vpath!("/dev/lvd2").to_owned());
    opts.insert("ob:root", args.game);

    if let Err(e) = fs.mount(opts, MountFlags::empty(), None) {
        return Err(KernelError::MountFailed(game, e));
    }

    // TODO: Check permission of /mnt/sandbox/CUSAXXXXX_000 on the PS4.
    let proc_root: VPathBuf = format!("/mnt/sandbox/{}_000", param.title_id())
        .try_into()
        .unwrap();

    if let Err(e) = fs.mkdir(&proc_root, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(proc_root, e));
    }

    // TODO: Check permission of /mnt/sandbox/CUSAXXXXX_000/app0 on the PS4.
    let app = proc_root.join("app0").unwrap();

    if let Err(e) = fs.mkdir(&app, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(app, e));
    }

    // TODO: Get mount options from the PS4.
    let mut opts = MountOpts::new();

    opts.insert("fstype", "nullfs");
    opts.insert("fspath", app.clone());
    opts.insert("target", game);

    if let Err(e) = fs.mount(opts, MountFlags::empty(), None) {
        return Err(KernelError::MountFailed(app, e));
    }

    let system_component = "QXuNNl0Zhn";

    let system_path = proc_root.join(system_component).unwrap();

    if let Err(e) = fs.mkdir(&system_path, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(system_path, e));
    }

    // TODO: Check permission of /mnt/sandbox/CUSAXXXXX_000/<SYSTEM_PATH>/common on the PS4.
    let common_path = system_path.join("common").unwrap();

    if let Err(e) = fs.mkdir(&common_path, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(common_path, e));
    }

    // TODO: Check permission of /mnt/sandbox/CUSAXXXXX_000/<SYSTEM_PATH>/common/lib on the PS4.
    let lib_path = common_path.join("lib").unwrap();

    if let Err(e) = fs.mkdir(&lib_path, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(lib_path, e));
    }

    // TODO: Get mount options from the PS4.
    let mut opts = MountOpts::new();

    opts.insert("fstype", "nullfs");
    opts.insert("fspath", lib_path);
    opts.insert("target", vpath!("/system/common/lib").to_owned());

    if let Err(e) = fs.mount(opts, MountFlags::empty(), None) {
        return Err(KernelError::MountFailed(app, e));
    }

    // TODO: Check permission of /mnt/sandbox/pfsmnt/CUSAXXXXX-app0-patch0-union on the PS4.
    let path: VPathBuf = format!("/mnt/sandbox/pfsmnt/{}-app0-patch0-union", param.title_id())
        .try_into()
        .unwrap();

    if let Err(e) = fs.mkdir(&path, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(path, e));
    }

    // TODO: Get mount options from the PS4.
    let mut opts = MountOpts::new();

    opts.insert("fstype", "nullfs");
    opts.insert("fspath", path.clone());
    opts.insert("target", app);

    if let Err(e) = fs.mount(opts, MountFlags::empty(), None) {
        return Err(KernelError::MountFailed(path, e));
    }

    // TODO: Check permission of /mnt/sandbox/CUSAXXXXX_000/dev on the PS4.
    let path = proc_root.join("dev").unwrap();

    if let Err(e) = fs.mkdir(&path, 0o555, None) {
        return Err(KernelError::CreateDirectoryFailed(path, e));
    }

    // TODO: Get mount options from the PS4.
    let mut opts = MountOpts::new();

    opts.insert("fstype", "devfs");
    opts.insert("fspath", path.clone());

    if let Err(e) = fs.mount(opts, MountFlags::empty(), None) {
        return Err(KernelError::MountFailed(path, e));
    }

    // Initialize TTY system.
    #[allow(unused_variables)] // TODO: Remove this when someone uses tty.
    let tty = TtyManager::new()?;

    // Initialize kernel components.
    #[allow(unused_variables)] // TODO: Remove this when someone uses debug.
    let debug = DebugManager::new()?;
    RegMgr::new(&mut syscalls);
    let machdep = MachDep::new(&mut syscalls);
    let budget = BudgetManager::new(&mut syscalls);

    DmemManager::new(&fs, &mut syscalls)?;
    SharedMemoryManager::new(&mut syscalls);
    Sysctl::new(&machdep, &mut syscalls);
    TimeManager::new(&mut syscalls);
    KernelQueueManager::new(&mut syscalls);
    NetManager::new(&mut syscalls);
    NamedObjManager::new(&mut syscalls);
    OsemManager::new(&mut syscalls);
    UmtxManager::new(&mut syscalls);

    // Initialize runtime linker.
    info!("Initializing runtime linker.");

    let ee = NativeEngine::new();
    let ld = RuntimeLinker::new(&fs, &ee, &mut syscalls);

    // TODO: Get correct budget name from the PS4.
    let budget_id = budget.create(Budget::new("big app", ProcType::BigApp));
    let proc_root = fs.lookup(proc_root, true, None).unwrap();

    let proc = VProc::new(
        auth,
        budget_id,
        ProcType::BigApp,
        dev::DmemContainer::One, // See sys_budget_set on the PS4.
        proc_root,
        system_component,
        syscalls,
    )?;

    info!(
        "Application stack: {:p}:{:p}",
        proc.vm().stack().start(),
        proc.vm().stack().end()
    );

    // TODO: Check if this credential is actually correct for game main thread.
    let cred = Arc::new(Ucred::new(
        Uid::ROOT,
        Uid::ROOT,
        vec![Gid::ROOT],
        AuthInfo::SYS_CORE.clone(),
    ));

    let main = VThread::new(proc.clone(), &cred);

    // Load eboot.bin.
    let path = vpath!("/app0/eboot.bin");
    let app = ld
        .exec(path, &main)
        .map_err(|e| KernelError::ExecFailed(path, e))?;

    let mut log = info!();

    writeln!(log, "Application   : {}", app.path()).unwrap();
    app.print(log);

    let lib_path = VPathBuf::new()
        .join(system_component)
        .unwrap()
        .join("common")
        .unwrap()
        .join("lib")
        .unwrap();

    // Preload libkernel.
    let mut flags = LoadFlags::UNK1;
    let path = lib_path.join("libkernel.sprx").unwrap();

    if proc.budget_ptype() == ProcType::BigApp {
        flags |= LoadFlags::BIG_APP;
    }

    info!("Loading {path}.");

    let (libkernel, _) = ld
        .load(path, flags, false, true, &main)
        .map_err(|e| KernelError::FailedToLoadLibkernel(e.into()))?;

    libkernel.flags_mut().remove(ModuleFlags::UNK2);
    libkernel.print(info!());

    ld.set_kernel(libkernel);

    // Preload libSceLibcInternal.
    let path = lib_path.join("libSceLibcInternal.sprx").unwrap();

    info!("Loading {path}.");

    let (libc, _) = ld
        .load(path, flags, false, true, &main)
        .map_err(|e| KernelError::FailedToLoadLibSceLibcInternal(e.into()))?;

    libc.flags_mut().remove(ModuleFlags::UNK2);
    libc.print(info!());

    drop(libc);

    // Get eboot.bin.
    if app.file_info().is_none() {
        todo!("statically linked eboot.bin");
    }

    // TODO: Setup hypervisor.

    // Get entry point.
    let boot = ld.kernel().unwrap();
    let mut arg = Box::pin(EntryArg::new(&proc, app.clone()));
    let entry = unsafe { boot.get_function(boot.entry().unwrap()) };
    let entry = move || unsafe { entry.exec1(arg.as_mut().as_vec().as_ptr()) };

    // Start main thread.
    info!("Starting application.");

    // TODO: Check how this constructed.
    let stack = proc.vm().stack();
    let main: OsThread = unsafe { main.start(stack.start(), stack.len(), entry) }?;

    // Begin Discord Rich Presence before blocking current thread.
    if let Err(e) = discord_presence(&param) {
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
    FilesystemInitFailed(#[from] FsInitError),

    #[error("couldn't create {0}")]
    CreateDirectoryFailed(VPathBuf, #[source] MkdirError),

    #[error("couldn't mount {0}")]
    MountFailed(VPathBuf, #[source] MountError),

    #[error("tty initialization failed")]
    TtyInitFailed(#[from] TtyInitError),

    #[error("debug manager initialization failed")]
    DebugManagerInitFailed(#[from] DebugManagerInitError),

    #[error("dmem manager initialization failed")]
    DmemManagerInitFailes(#[from] DmemManagerInitError),

    #[error("virtual process initialization failed")]
    VProcInitFailed(#[from] VProcInitError),

    #[error("couldn't execute {0}")]
    ExecFailed(&'static VPath, #[source] ExecError),

    #[error("libkernel couldn't be loaded")]
    FailedToLoadLibkernel(#[source] Box<dyn Error>),

    #[error("libSceLibcInternal couldn't be loaded")]
    FailedToLoadLibSceLibcInternal(#[source] Box<dyn Error>),

    #[error("couldn't create a hypervisor")]
    CreateHypervisorFailed(#[from] hv::HypervisorError),

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
