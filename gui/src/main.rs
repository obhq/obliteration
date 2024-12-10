#![windows_subsystem = "windows"]

use self::data::{DataError, DataMgr};
use self::graphics::{Graphics, GraphicsError, PhysicalDevice};
use self::profile::Profile;
use self::setup::{run_setup, SetupError};
use self::ui::{ErrorWindow, MainWindow, ProfileModel, ResolutionModel, SlintBackend};
use clap::{Parser, ValueEnum};
use debug::DebugServer;
use erdp::ErrorDisplay;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::Cell;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

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

    // Setup UI event loop.
    let mut el = EventLoop::<ProgramEvent>::with_user_event();
    let el = match el.build() {
        Ok(v) => v,
        Err(e) => {
            error(format!(
                "Failed to create winit event loop: {}.",
                e.display()
            ));

            return ExitCode::FAILURE;
        }
    };

    // Run.
    let mut prog = Program { args };

    match el.run_app(&mut prog) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error(format!("Failed to run winit event loop: {}.", e.display()));
            ExitCode::FAILURE
        }
    }
}

fn run_vmm(args: &ProgramArgs, exe: &PathBuf) -> Result<(), ProgramError> {
    #[cfg(unix)]
    rlim::set_rlimit_nofile().map_err(ProgramError::FdLimit)?;

    // Initialize graphics engine.
    let mut graphics = graphics::new().map_err(ProgramError::InitGraphics)?;

    // Run setup wizard. This will simply return the data manager if the user already has required
    // settings.
    let data = match run_setup().map_err(ProgramError::Setup)? {
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

    for l in data.prof().list().map_err(ProgramError::ListProfile)? {
        let l = l.map_err(ProgramError::ListProfile)?;
        let p = Profile::load(&l).map_err(ProgramError::LoadProfile)?;

        profiles.push(p);
    }

    // Create default profile if user does not have any profiles.
    if profiles.is_empty() {
        // Create directory.
        let p = Profile::default();
        let l = data.prof().data(p.id());

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
            DebugServer::new(listen).map_err(|e| ProgramError::StartDebugServer(e, listen))?;

        let debugger = debug_server
            .accept()
            .map_err(ProgramError::CreateDebugClient)?;

        Some(debugger)
    } else {
        None
    };

    // Setup VMM screen.
    let screen = graphics
        .create_screen(&profile)
        .map_err(|e| ProgramError::CreateScreen(Box::new(e)))?;

    todo!()
}

fn run_launcher(
    graphics: &impl Graphics,
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
    win.run().map_err(ProgramError::RunMainWindow)?;

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

/// Implementation of [`ApplicationHandler`] for main program mode.
struct Program {
    args: ProgramArgs,
}

impl Program {
    async fn error(&self, msg: impl Into<SharedString>) {
        // Show error window.
        let win = ErrorWindow::new().unwrap();

        win.set_message(msg.into());
        win.on_close({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        win.show();

        todo!()
    }
}

impl ApplicationHandler<ProgramEvent> for Program {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        slint::platform::set_platform(Box::new(SlintBackend::new())).unwrap();

        todo!()
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        todo!()
    }
}

/// Event to wakeup UI event loop.
enum ProgramEvent {}

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
}
