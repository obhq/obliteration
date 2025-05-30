use crate::arch::MachDep;
use crate::args::Args;
use crate::budget::{Budget, BudgetManager, ProcType};
use crate::dev::{
    CameraInitError, CameraManager, DceInitError, DceManager, DebugManager, DebugManagerInitError,
    DipswInitError, DipswManager, DmemContainer, GcInitError, GcManager, HmdInitError, HmdManager,
    RngInitError, RngManager, SblSrvInitError, SblSrvManager, TtyManager, TtyManagerInitError,
};
use crate::dmem::{DmemManager, DmemManagerInitError};
use crate::ee::native::NativeEngine;
use crate::ee::EntryArg;
use crate::errno::EEXIST;
use crate::fs::{Fs, FsInitError, MkdirError, MountError, MountFlags, MountOpts, VPath, VPathBuf};
use crate::kqueue::KernelQueueManager;
use crate::namedobj::NamedObjManager;
use crate::net::NetManager;
use crate::osem::OsemManager;
use crate::process::{ProcManager, ProcManagerError};
use crate::rcmgr::RcMgr;
use crate::regmgr::RegMgr;
use crate::rtld::{ExecError, LoadFlags, ModuleFlags, RuntimeLinker};
use crate::sched::Scheduler;
use crate::shm::SharedMemoryManager;
use crate::signal::SignalManager;
use crate::syscalls::Syscalls;
use crate::sysctl::Sysctl;
use crate::sysent::ProcAbi;
use crate::time::TimeManager;
use crate::ucred::{AuthAttrs, AuthCaps, AuthInfo, AuthPaid, Gid, Ucred, Uid};
use crate::umtx::UmtxManager;
use crate::vm::VmMgr;
use llt::{OsThread, SpawnError};
use macros::vpath;
use param::Param;
use std::error::Error;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::{MemoryRefreshKind, System};
use thiserror::Error;

mod arch;
mod args;
mod arnd;
mod budget;
mod dev;
mod dmem;
mod ee;
mod errno;
mod event;
mod fs;
mod idps;
mod idt;
mod imgact;
mod ipmi;
mod kqueue;
mod namedobj;
mod net;
mod osem;
mod pcpu;
mod process;
mod rcmgr;
mod regmgr;
mod rtld;
mod sched;
mod shm;
mod signal;
mod subsystem;
mod syscalls;
mod sysctl;
mod sysent;
mod time;
mod ucred;
mod umtx;
mod vm;

fn run(args: Args) -> Result<(), KernelError> {
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
    let mut sys = Syscalls::new();
    let sched = Arc::new(Scheduler::new());
    let vm = VmMgr::new(&mut sys);
    let fs = Fs::new(args.system, &cred, &mut sys).map_err(KernelError::FilesystemInitFailed)?;
    let rc = RcMgr::new();
    let pmgr = ProcManager::new(&cred, &fs, &rc, &mut sys)
        .map_err(KernelError::CreateProcManagerFailed)?;

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
    let tty = TtyManager::new().map_err(KernelError::TtyInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses dipsw.
    let dipsw = DipswManager::new().map_err(KernelError::DipswInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses gc.
    let gc = GcManager::new().map_err(KernelError::GcManagerInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses camera.
    let camera = CameraManager::new().map_err(KernelError::CameraManagerInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses rng.
    let rng = RngManager::new().map_err(KernelError::RngManagerInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses sbl_srv.
    let sbl_srv = SblSrvManager::new().map_err(KernelError::SblSrvManagerInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses hmd.
    let hmd_cmd = HmdManager::new().map_err(KernelError::HmdManagerInitFailed)?;
    #[allow(unused_variables)] // TODO: Remove this when someone uses dce.
    let dce = DceManager::new().map_err(KernelError::DceManagerInitFailed)?;

    // Initialize kernel components.
    #[allow(unused_variables)] // TODO: Remove this when someone uses debug.
    let debug = DebugManager::new().map_err(KernelError::DebugManagerInitFailed)?;
    RegMgr::new(&mut sys);
    let machdep = MachDep::new(&mut sys);
    let budget = BudgetManager::new(&mut sys);

    SignalManager::new(&mut sys);
    DmemManager::new(&fs, &mut sys).map_err(KernelError::DmemManagerInitFailed)?;
    SharedMemoryManager::new(&mut sys);
    Sysctl::new(&machdep, &mut sys);
    TimeManager::new(&mut sys);
    KernelQueueManager::new(&mut sys);
    NetManager::new(&mut sys);
    NamedObjManager::new(&mut sys);
    OsemManager::new(&mut sys);
    UmtxManager::new(&mut sys);

    // Initialize runtime linker.
    let ee = NativeEngine::new();
    let ld = RuntimeLinker::new(&fs, &ee, &mut sys);

    // TODO: Get correct budget name from the PS4.
    let sys = Arc::new(sys);
    let budget_id = budget.create(Budget::new("big app", ProcType::BigApp));
    let proc_root = fs.lookup(proc_root, true, None).unwrap();
    let main = pmgr
        .spawn(
            ProcAbi::new(Some(sys)),
            auth,
            budget_id,
            ProcType::BigApp,
            DmemContainer::One, // See sys_budget_set on the PS4.
            proc_root,
            system_component,
            true, // TODO: Change to false when we switched to run /mini-syscore.elf.
        )
        .map_err(KernelError::CreateProcessFailed)?;
    let proc = main.proc();

    info!(
        "Application stack: {:p}:{:p}",
        proc.vm_space().stack().start(),
        proc.vm_space().stack().end()
    );

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

    if proc.budget_ptype().is_some_and(|v| v == ProcType::BigApp) {
        flags |= LoadFlags::BIG_APP;
    }

    info!("Loading {path}.");

    let (libkernel, _) = ld
        .load(path, flags, false, true, &main)
        .map_err(|e| KernelError::FailedToLoadLibkernel(e.into()))?;

    libkernel.flags_mut().remove(ModuleFlags::IS_NEW);
    libkernel.print(info!());

    ld.set_kernel(libkernel);

    // Preload libSceLibcInternal.
    let path = lib_path.join("libSceLibcInternal.sprx").unwrap();

    info!("Loading {path}.");

    let (libc, _) = ld
        .load(path, flags, false, true, &main)
        .map_err(|e| KernelError::FailedToLoadLibSceLibcInternal(e.into()))?;

    libc.flags_mut().remove(ModuleFlags::IS_NEW);
    libc.print(info!());

    drop(libc);

    // Get eboot.bin.
    if app.file_info().is_none() {
        todo!("statically linked eboot.bin");
    }

    // Get entry point.
    let boot = ld.kernel().unwrap();
    let mut arg = Box::pin(EntryArg::new(&proc, app.clone()));
    let entry = unsafe { boot.get_function(boot.entry().unwrap()) };
    let entry = move || unsafe { entry.exec1(arg.as_mut().as_vec().as_ptr()) };

    // Start main thread.
    info!("Starting application.");

    // TODO: Check how this constructed.
    let stack = proc.vm_space().stack();
    let main = unsafe { main.start(stack.start(), stack.len(), entry) }?;

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

#[derive(Debug, Error)]
enum DiscordPresenceError {
    #[error("failed to create Discord IPC")]
    FailedToCreateIpc(#[source] Box<dyn Error>),

    #[error("failed to update Discord presence")]
    FailedToUpdatePresence(#[source] Box<dyn Error>),
}

/// Represents an error when [`run()`] fails.
#[derive(Debug, Error)]
enum KernelError {
    #[error("couldn't open param.sfo")]
    FailedToOpenGameParam(#[source] std::io::Error),

    #[error("couldn't read param.sfo ")]
    FailedToReadGameParam(#[source] param::ReadError),

    #[error("{0} has an invalid title identifier")]
    InvalidTitleId(PathBuf),

    #[error("filesystem initialization failed")]
    FilesystemInitFailed(#[source] FsInitError),

    #[error("couldn't create a process manager")]
    CreateProcManagerFailed(#[source] ProcManagerError),

    #[error("couldn't create {0}")]
    CreateDirectoryFailed(VPathBuf, #[source] MkdirError),

    #[error("couldn't mount {0}")]
    MountFailed(VPathBuf, #[source] MountError),

    #[error("tty initialization failed")]
    TtyInitFailed(#[source] TtyManagerInitError),

    #[error("dipsw initialization failed")]
    DipswInitFailed(#[source] DipswInitError),

    #[error("debug manager initialization failed")]
    DebugManagerInitFailed(#[source] DebugManagerInitError),

    #[error("gc manager initialization failed")]
    GcManagerInitFailed(#[source] GcInitError),

    #[error("camera manager initialization failed")]
    CameraManagerInitFailed(#[source] CameraInitError),

    #[error("rng manager initialization failed")]
    RngManagerInitFailed(#[source] RngInitError),

    #[error("dmem manager initialization failed")]
    DmemManagerInitFailed(#[source] DmemManagerInitError),

    #[error("sbl_srv manager initialization failed")]
    SblSrvManagerInitFailed(#[source] SblSrvInitError),

    #[error("hmd manager initialization failed")]
    HmdManagerInitFailed(#[source] HmdInitError),

    #[error("dce manager initialization failed")]
    DceManagerInitFailed(#[source] DceInitError),

    #[error("couldn't create application process")]
    CreateProcessFailed(#[source] self::process::SpawnError),

    #[error("couldn't execute {0}")]
    ExecFailed(&'static VPath, #[source] ExecError),

    #[error("libkernel couldn't be loaded")]
    FailedToLoadLibkernel(#[source] self::rtld::LoadError),

    #[error("libSceLibcInternal couldn't be loaded")]
    FailedToLoadLibSceLibcInternal(#[source] self::rtld::LoadError),

    #[error("main thread couldn't be created")]
    FailedToCreateMainThread(#[from] SpawnError),

    #[error("failed to join with main thread")]
    FailedToJoinMainThread(#[source] std::io::Error),
}
