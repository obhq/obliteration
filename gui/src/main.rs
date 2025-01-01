#![windows_subsystem = "windows"]

use self::data::{DataError, DataMgr};
use self::graphics::{EngineBuilder, GraphicsError, PhysicalDevice};
use self::log::LogWriter;
use self::profile::{DisplayResolution, Profile};
use self::setup::{run_setup, SetupError};
use self::ui::{
    MainWindow, PlatformExt, ProfileModel, ResolutionModel, RuntimeExt, SlintBackend,
    WaitForDebugger,
};
use self::vmm::{CpuError, Vmm, VmmError, VmmEvent};
use async_net::{TcpListener, TcpStream};
use clap::{Parser, ValueEnum};
use erdp::ErrorDisplay;
use futures::{select_biased, AsyncReadExt, FutureExt};
use slint::{ComponentHandle, ModelRc, SharedString, ToSharedString, VecModel};
use std::cell::Cell;
use std::future::Future;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::pin::pin;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Poll;
use thiserror::Error;
use winit::dpi::PhysicalSize;
use winit::window::Window;

mod data;
mod dialogs;
mod gdb;
mod graphics;
mod hv;
mod log;
mod panic;
mod profile;
mod rt;
mod setup;
mod ui;
mod vfs;
mod vmm;

fn main() -> ExitCode {
    // Check program mode.
    let args = ProgramArgs::parse();

    match &args.mode {
        Some(ProgramMode::PanicHandler) => return self::panic::run_handler(),
        None => {}
    }

    #[cfg(target_os = "windows")]
    fn error(msg: impl AsRef<str>) {
        todo!()
    }

    #[cfg(not(target_os = "windows"))]
    fn error(msg: impl AsRef<str>) {
        eprintln!("{}", msg.as_ref());
    }

    // Spawn panic handler.
    let exe = match std::env::current_exe().and_then(std::fs::canonicalize) {
        Ok(v) => v,
        Err(e) => {
            error(format!(
                "Failed to get application executable path: {}.",
                e.display()
            ));

            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = self::panic::spawn_handler(&exe) {
        error(format!(
            "Failed to spawn panic handler process: {}.",
            e.display()
        ));

        return ExitCode::FAILURE;
    }

    // Run.
    let main = async move {
        // Setup Slint custom back-end. This need to be done before using any Slint API.
        match unsafe { SlintBackend::new() } {
            Ok(v) => v.install().unwrap(), // This should never fail.
            Err(e) => {
                error(format!(
                    "Failed to initialize Slint back-end: {}.",
                    e.display()
                ));

                return ExitCode::FAILURE;
            }
        }

        // Run.
        let e = match run(args, exe).await {
            Ok(_) => return ExitCode::SUCCESS,
            Err(e) => e,
        };

        // Show error window.
        let msg = format!("An unexpected error has occurred: {}.", e.display());

        self::ui::error(msg).await;

        ExitCode::FAILURE
    };

    match self::rt::run(main) {
        Ok(v) => v,
        Err(e) => {
            error(format!(
                "Failed to run application runtime: {}.",
                e.display()
            ));

            ExitCode::FAILURE
        }
    }
}

async fn run(args: ProgramArgs, exe: PathBuf) -> Result<(), ProgramError> {
    // Increase number of file descriptor to maximum allowed.
    #[cfg(unix)]
    unsafe {
        use libc::{getrlimit, setrlimit, RLIMIT_NOFILE};
        use std::io::Error;
        use std::mem::MaybeUninit;

        // Get current value.
        let mut val = MaybeUninit::uninit();

        if getrlimit(RLIMIT_NOFILE, val.as_mut_ptr()) < 0 {
            return Err(ProgramError::GetFdLimit(Error::last_os_error()));
        }

        // Check if we need to increase the limit.
        let mut val = val.assume_init();

        if val.rlim_cur < val.rlim_max {
            val.rlim_cur = val.rlim_max;

            if setrlimit(RLIMIT_NOFILE, &val) < 0 {
                return Err(ProgramError::SetFdLimit(Error::last_os_error()));
            }
        }
    }

    // Initialize graphics engine.
    let graphics = graphics::builder().map_err(ProgramError::InitGraphics)?;

    // Run setup wizard. This will simply return the data manager if the user already has required
    // settings.
    let data = match run_setup().await.map_err(ProgramError::Setup)? {
        Some(v) => Arc::new(v),
        None => return Ok(()),
    };

    // Get kernel path.
    let kernel = args.kernel.as_ref().cloned().unwrap_or_else(|| {
        // Get kernel directory.
        let mut path = exe.parent().unwrap().to_owned();

        #[cfg(target_os = "windows")]
        path.push("share");

        #[cfg(not(target_os = "windows"))]
        {
            path.pop();

            #[cfg(target_os = "macos")]
            path.push("Resources");

            #[cfg(not(target_os = "macos"))]
            path.push("share");
        }

        // Append kernel.
        path.push("obkrnl");
        path
    });

    // Load profiles.
    let mut profiles = Vec::new();

    for l in data.profiles().list().map_err(ProgramError::ListProfile)? {
        let l = l.map_err(ProgramError::ListProfile)?;
        let p = Profile::load(&l).map_err(ProgramError::LoadProfile)?;

        profiles.push(p);
    }

    // Create default profile if user does not have any profiles.
    if profiles.is_empty() {
        // Create directory.
        let p = Profile::default();
        let l = data.profiles().data(p.id());

        if let Err(e) = std::fs::create_dir(&l) {
            return Err(ProgramError::CreateDirectory(l, e));
        }

        // Save.
        p.save(&l).map_err(ProgramError::SaveDefaultProfile)?;

        profiles.push(p);
    }

    // Get profile to use.
    let (profile, debug) = if let Some(v) = args.debug {
        // TODO: Select last used profile.
        (profiles.pop().unwrap(), Some(v))
    } else {
        let (profile, exit) = match run_launcher(&graphics, &data, profiles).await? {
            Some(v) => v,
            None => return Ok(()),
        };

        match exit {
            ExitAction::Run => (profile, None),
            ExitAction::RunDebug(v) => (profile, Some(v)),
        }
    };

    // Wait for debugger.
    let mut gdb_con = if let Some(addr) = debug {
        let v = wait_for_debugger(addr).await?;

        if v.is_none() {
            return Ok(());
        }

        v
    } else {
        None
    };

    // Setup WindowAttributes for VMM screen.
    let attrs = Window::default_attributes()
        .with_inner_size(match profile.display_resolution() {
            DisplayResolution::Hd => PhysicalSize::new(1280, 720),
            DisplayResolution::FullHd => PhysicalSize::new(1920, 1080),
            DisplayResolution::UltraHd => PhysicalSize::new(3840, 2160),
        })
        .with_resizable(false)
        .with_title("Obliteration");

    // Prepare to launch VMM.
    let logs = data.logs();
    let mut logs =
        LogWriter::new(logs).map_err(|e| ProgramError::CreateKernelLog(logs.into(), e))?;
    let shutdown = Arc::default();
    let graphics = graphics
        .build(&profile, attrs, &shutdown)
        .map_err(ProgramError::BuildGraphicsEngine)?;
    let mut gdb_in = [0; 1024];

    // Start VMM.
    let mut vmm = match Vmm::new(&profile, &kernel, None, &shutdown) {
        Ok(v) => v,
        Err(e) => return Err(ProgramError::StartVmm(kernel, e)),
    };

    loop {
        // Prepare futures to poll.
        let mut vmm = pin!(vmm.recv());
        let mut debug = gdb_con.as_mut().map(|v| v.read(&mut gdb_in));

        // Poll all futures.
        let (vmm, debug) = std::future::poll_fn(move |cx| {
            let vmm = vmm.as_mut().poll(cx);
            let debug = debug.as_mut().map_or(Poll::Pending, |d| d.poll_unpin(cx));

            match (vmm, debug) {
                (Poll::Ready(v), Poll::Ready(d)) => Poll::Ready((Some(v), Some(d))),
                (Poll::Ready(v), Poll::Pending) => Poll::Ready((Some(v), None)),
                (Poll::Pending, Poll::Ready(d)) => Poll::Ready((None, Some(d))),
                (Poll::Pending, Poll::Pending) => Poll::Pending,
            }
        })
        .await;

        // Process VMM event.
        if let Some(vmm) = vmm {
            match vmm {
                VmmEvent::Exit(id, r) => {
                    if !r.map_err(ProgramError::CpuThread)? {
                        return Err(ProgramError::CpuPanic(id, logs.path().into()));
                    } else if id == 0 {
                        break;
                    }
                }
                VmmEvent::Log(t, m) => logs.write(t, m),
            }
        }

        // Process debugger requests.
        if let Some(debug) = debug {
            todo!()
        }
    }

    Ok(())
}

async fn run_launcher(
    graphics: &impl EngineBuilder,
    data: &Arc<DataMgr>,
    profiles: Vec<Profile>,
) -> Result<Option<(Profile, ExitAction)>, ProgramError> {
    // Create window and register callback handlers.
    let win = MainWindow::new().map_err(ProgramError::CreateMainWindow)?;
    let resolutions = Rc::new(ResolutionModel::default());
    let profiles = Rc::new(ProfileModel::new(profiles, resolutions.clone()));
    let exit = Rc::new(Cell::new(None));

    win.on_profile_selected({
        let win = win.as_weak();
        let profiles = profiles.clone();

        move || {
            // TODO: Check if previous profile has unsaved data before switch the profile.
            let win = win.unwrap();
            let row: usize = win.get_selected_profile().try_into().unwrap();

            profiles.select(row, &win);
        }
    });

    win.on_save_profile({
        let data = data.clone();
        let win = win.as_weak();
        let profiles = profiles.clone();

        move || {
            let win = win.unwrap();
            let row = win.get_selected_profile();
            let pro = profiles.update(row, &win);
            let loc = data.profiles().data(pro.id());

            // TODO: Display error instead of panic.
            pro.save(loc).unwrap();
        }
    });

    win.on_report_issue(|| {
        // TODO: Display error instead of panic.
        open::that_detached("https://github.com/obhq/obliteration/issues/new").unwrap();
    });

    win.on_start_vmm({
        let win = win.as_weak();
        let exit = exit.clone();

        move || {
            win.unwrap().hide().unwrap();
            exit.set(Some(ExitAction::Run));
        }
    });

    win.on_start_debug({
        let win = win.as_weak();
        let exit = exit.clone();

        move |addr| {
            let addr = match addr.parse() {
                Ok(addr) => addr,
                // TODO: Display error instead of panic.
                Err(_e) => todo!(),
            };

            win.unwrap().hide().unwrap();

            exit.set(Some(ExitAction::RunDebug(addr)));
        }
    });

    // Set window properties.
    let physical_devices = ModelRc::new(VecModel::from_iter(
        graphics
            .physical_devices()
            .iter()
            .map(|p| SharedString::from(p.name())),
    ));

    win.set_devices(physical_devices);
    win.set_resolutions(resolutions.into());
    win.set_profiles(profiles.clone().into());

    // Load selected profile.
    let row: usize = win.get_selected_profile().try_into().unwrap();

    profiles.select(row, &win);

    // Run the window.
    win.show().map_err(ProgramError::ShowMainWindow)?;
    win.set_center().map_err(ProgramError::CenterMainWindow)?;
    win.wait().await;

    // Update selected profile.
    let profile = win.get_selected_profile();

    profiles.update(profile, &win);

    drop(win);

    // Check how we exit.
    let exit = match Rc::into_inner(exit).unwrap().into_inner() {
        Some(v) => v,
        None => return Ok(None),
    };

    // Get selected profile.
    let mut profiles = Rc::into_inner(profiles).unwrap().into_inner();
    let profile = profiles.remove(profile.try_into().unwrap());

    Ok(Some((profile, exit)))
}

async fn wait_for_debugger(addr: SocketAddrV4) -> Result<Option<TcpStream>, ProgramError> {
    // Start server.
    let server = TcpListener::bind(addr)
        .await
        .map_err(|e| ProgramError::StartDebugServer(addr, e))?;
    let addr = server.local_addr().map_err(ProgramError::GetDebugAddr)?;

    // Tell the user that we are waiting for a debugger.
    let win = WaitForDebugger::new().map_err(ProgramError::CreateDebugWindow)?;

    win.set_address(addr.to_shared_string());
    win.show().map_err(ProgramError::ShowDebugWindow)?;

    // Wait for connection.
    let client = select_biased! {
        _ = win.wait().fuse() => return Ok(None),
        v = server.accept().fuse() => match v {
            Ok(v) => v.0,
            Err(e) => return Err(ProgramError::AcceptDebugger(e)),
        }
    };

    // Disable Nagle algorithm since it does not work well with GDB remote protocol.
    client
        .set_nodelay(true)
        .map_err(ProgramError::DisableDebuggerNagle)?;

    Ok(Some(client))
}

/// Program arguments parsed from command line.
#[derive(Parser)]
#[command(about = None)]
struct ProgramArgs {
    #[arg(long, value_enum, hide = true)]
    mode: Option<ProgramMode>,

    /// Immediate launch the VMM in debug mode.
    #[arg(long)]
    debug: Option<SocketAddrV4>,

    /// Use the kernel image at the specified path instead of the default one.
    #[arg(long)]
    kernel: Option<PathBuf>,
}

/// Action to be performed after the main window is closed.
enum ExitAction {
    Run,
    RunDebug(SocketAddrV4),
}

/// Mode of our program.
#[derive(Clone, ValueEnum)]
enum ProgramMode {
    PanicHandler,
}

/// Represents an error when our program fails.
#[derive(Debug, Error)]
enum ProgramError {
    #[cfg(unix)]
    #[error("couldn't get file descriptor limit")]
    GetFdLimit(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("couldn't increase file descriptor limit")]
    SetFdLimit(#[source] std::io::Error),

    #[error("couldn't run setup wizard")]
    Setup(#[source] SetupError),

    #[error("couldn't list available profiles")]
    ListProfile(#[source] DataError),

    #[error("couldn't load profile")]
    LoadProfile(#[source] self::profile::LoadError),

    #[error("couldn't create {0}")]
    CreateDirectory(PathBuf, #[source] std::io::Error),

    #[error("couldn't save default profile")]
    SaveDefaultProfile(#[source] self::profile::SaveError),

    #[error("couldn't start debug server on {0}")]
    StartDebugServer(SocketAddrV4, #[source] std::io::Error),

    #[error("couldn't get debug server address")]
    GetDebugAddr(#[source] std::io::Error),

    #[error("couldn't create debug server window")]
    CreateDebugWindow(#[source] slint::PlatformError),

    #[error("couldn't show debug server window")]
    ShowDebugWindow(#[source] slint::PlatformError),

    #[error("couldn't accept a connection from debugger")]
    AcceptDebugger(#[source] std::io::Error),

    #[error("couldn't disable Nagle algorithm on debugger connection")]
    DisableDebuggerNagle(#[source] std::io::Error),

    #[error("couldn't create main window")]
    CreateMainWindow(#[source] slint::PlatformError),

    #[error("couldn't initialize graphics engine")]
    InitGraphics(#[source] GraphicsError),

    #[error("couldn't center main window")]
    CenterMainWindow(#[source] self::ui::PlatformError),

    #[error("couldn't show main window")]
    ShowMainWindow(#[source] slint::PlatformError),

    #[error("couldn't create {0}")]
    CreateKernelLog(PathBuf, #[source] std::io::Error),

    #[error("couldn't build graphics engine")]
    BuildGraphicsEngine(#[source] GraphicsError),

    #[error("couldn't start VMM for {0}")]
    StartVmm(PathBuf, #[source] VmmError),

    #[error("thread for vCPU #{0} was stopped unexpectedly")]
    CpuThread(#[source] CpuError),

    #[error("vCPU #{0} panicked, see {1} for more information")]
    CpuPanic(usize, PathBuf),
}
