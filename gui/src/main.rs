#![windows_subsystem = "windows"]

use self::data::{DataError, DataMgr};
use self::debug::DebugClient;
use self::graphics::{Graphics, GraphicsError, PhysicalDevice, Screen};
use self::profile::Profile;
use self::setup::{run_setup, SetupError};
use self::ui::{ErrorWindow, MainWindow, ProfileModel, ResolutionModel};
use self::vmm::{Vmm, VmmError};
use clap::{Parser, ValueEnum};
use debug::DebugServer;
use gdbstub::stub::MultiThreadStopReason;
use obconf::ConsoleType;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::Cell;
use std::error::Error;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

mod data;
mod debug;
mod dialogs;
mod graphics;
mod hv;
mod panic;
mod profile;
#[cfg(unix)]
mod rlim;
mod setup;
mod ui;
mod vfs;
mod vmm;

fn main() -> ExitCode {
    use std::fmt::Write;

    // Check program mode.
    let args = CliArgs::parse();
    let r = match &args.mode {
        Some(ProgramMode::PanicHandler) => self::panic::run_handler(),
        None => run_vmm(&args),
    };

    // Check program result.
    let e = match r {
        Ok(_) => return ExitCode::SUCCESS,
        Err(e) => e,
    };

    // Get full message.
    let mut msg = e.to_string();
    let mut src = e.source();

    while let Some(e) = src {
        write!(msg, " -> {e}").unwrap();
        src = e.source();
    }

    // Show error window.
    let win = ErrorWindow::new().unwrap();

    win.set_message(format!("An unexpected error has occurred: {msg}.").into());
    win.on_close({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.run().unwrap();

    ExitCode::FAILURE
}

fn run_vmm(args: &CliArgs) -> Result<(), ApplicationError> {
    // Spawn panic handler.
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(ApplicationError::GetCurrentExePath)?;

    self::panic::spawn_handler(&exe)?;

    #[cfg(unix)]
    rlim::set_rlimit_nofile().map_err(ApplicationError::FdLimit)?;

    // Initialize graphics engine.
    let mut graphics = graphics::new().map_err(ApplicationError::InitGraphics)?;

    // Run setup wizard. This will simply return the data manager if the user already has required
    // settings.
    let data = match run_setup().map_err(ApplicationError::Setup)? {
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

    for l in data.prof().list().map_err(ApplicationError::ListProfile)? {
        let l = l.map_err(ApplicationError::ListProfile)?;
        let p = Profile::load(&l).map_err(ApplicationError::LoadProfile)?;

        profiles.push(p);
    }

    // Create default profile if user does not have any profiles.
    if profiles.is_empty() {
        // Create directory.
        let p = Profile::default();
        let l = data.prof().data(p.id());

        if let Err(e) = std::fs::create_dir(&l) {
            return Err(ApplicationError::CreateDirectory(l, e));
        }

        // Save.
        p.save(&l).map_err(ApplicationError::SaveDefaultProfile)?;

        profiles.push(p);
    }

    // Get profile to use.
    let (profile, debug) = if let Some(v) = args.debug {
        // TODO: Select last used profile.
        (profiles.pop().unwrap(), Some(v))
    } else {
        let (profile, exit) = match run_launcher(&graphics, &data, profiles)? {
            Some(v) => v,
            None => return Ok(()),
        };

        match exit {
            ExitAction::Run => (profile, None),
            ExitAction::RunDebug(v) => (profile, Some(v)),
        }
    };

    // Wait for debugger.
    let debugger = if let Some(listen) = debug {
        let debug_server =
            DebugServer::new(listen).map_err(|e| ApplicationError::StartDebugServer(e, listen))?;

        let debugger = debug_server
            .accept()
            .map_err(ApplicationError::CreateDebugClient)?;

        Some(debugger)
    } else {
        None
    };

    // Setup VMM screen.
    let screen = graphics
        .create_screen(&profile)
        .map_err(|e| ApplicationError::CreateScreen(Box::new(e)))?;

    // Start VMM.
    std::thread::scope(|scope| {
        let vmm = Vmm::new(
            VmmArgs {
                profile: &profile,
                kernel,
                debugger,
            },
            VmmHandler {},
            scope,
        )
        .map_err(ApplicationError::StartVmm)?;

        // Run the screen.
        screen
            .run()
            .map_err(|e| ApplicationError::RunScreen(Box::new(e)))?;

        Ok(())
    })
}

fn run_launcher(
    graphics: &impl Graphics,
    data: &Arc<DataMgr>,
    profiles: Vec<Profile>,
) -> Result<Option<(Profile, ExitAction)>, ApplicationError> {
    // Create window and register callback handlers.
    let win = MainWindow::new().map_err(ApplicationError::CreateMainWindow)?;
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
            let loc = data.prof().data(pro.id());

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
    win.run().map_err(ApplicationError::RunMainWindow)?;

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

/// Program arguments parsed from command line.
#[derive(Parser)]
#[command(about = None)]
struct CliArgs {
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

/// Encapsulates arguments for [`Vmm::new()`].
struct VmmArgs<'a> {
    profile: &'a Profile,
    kernel: PathBuf,
    debugger: Option<DebugClient>,
}

/// Provides method to handle VMM event.
struct VmmHandler {}

impl self::vmm::VmmHandler for VmmHandler {
    fn error(&self, cpu: usize, reason: impl Into<Box<dyn Error>>) {
        todo!()
    }

    fn exiting(&self, success: bool) {
        todo!()
    }

    fn log(&self, ty: ConsoleType, msg: &str) {
        todo!()
    }

    fn breakpoint(&self, stop: Option<MultiThreadStopReason<u64>>) {
        todo!()
    }
}

/// Mode of our program.
#[derive(Clone, ValueEnum)]
enum ProgramMode {
    PanicHandler,
}

/// Represents an error when [`run()`] fails.
#[derive(Debug, Error)]
enum ApplicationError {
    #[error("couldn't spawn panic handler process")]
    SpawnPanicHandler(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("couldn't increase file descriptor limit")]
    FdLimit(#[source] self::rlim::RlimitError),

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

    #[error("couldn't get application executable path")]
    GetCurrentExePath(#[source] std::io::Error),

    #[error("failed to start debug server on {1}")]
    StartDebugServer(
        #[source] debug::StartDebugServerError,
        std::net::SocketAddrV4,
    ),

    #[error("failed to accept debug connection")]
    CreateDebugClient(#[source] std::io::Error),

    #[error("failed to create main window")]
    CreateMainWindow(#[source] slint::PlatformError),

    #[error("couldn't initialize graphics engine")]
    InitGraphics(#[source] GraphicsError),

    #[error("failed to run main window")]
    RunMainWindow(#[source] slint::PlatformError),

    #[error("couldn't create VMM screen")]
    CreateScreen(#[source] Box<dyn std::error::Error>),

    #[error("couldn't start VMM")]
    StartVmm(#[source] VmmError),

    #[error("couldn't run VMM screen")]
    RunScreen(#[source] Box<dyn std::error::Error>),

    #[error("couldn't read panic info")]
    ReadPanicInfo(#[source] ciborium::de::Error<std::io::Error>),
}
