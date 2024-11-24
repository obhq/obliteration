#![windows_subsystem = "windows"]

use self::graphics::{Graphics, PhysicalDevice, Screen};
use self::profile::Profile;
use self::setup::{run_setup, SetupError};
use self::ui::ErrorDialog;
use args::CliArgs;
use clap::Parser;
use debug::DebugServer;
use slint::{ComponentHandle, Global, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use thiserror::Error;

mod args;
mod debug;
mod error;
mod graphics;
mod hv;
mod param;
mod profile;
#[cfg(unix)]
mod rlim;
mod setup;
mod string;
mod system;
mod ui;
mod vmm;

fn main() -> ExitCode {
    match run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            display_error(e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), ApplicationError> {
    let args = CliArgs::try_parse().map_err(ApplicationError::ParseArgs)?;

    #[cfg(unix)]
    rlim::set_rlimit_nofile().map_err(ApplicationError::FdLimit)?;

    // Initialize graphics engine.
    let mut graphics = match graphics::DefaultApi::new() {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::InitGraphics(Box::new(e))),
    };

    // Run setup wizard. This will do nothing if the user already has required settings.
    run_setup().map_err(ApplicationError::Setup)?;

    // Get VMM arguments.
    let args = if let Some(debug_addr) = args.debug_addr() {
        let kernel_path = get_kernel_path(&args)?;

        let debug_server = DebugServer::new(debug_addr)
            .map_err(|e| ApplicationError::StartDebugServer(e, debug_addr))?;

        let debug_client = debug_server
            .accept()
            .map_err(ApplicationError::CreateDebugClient)?;

        let profiles = load_profiles()?;

        todo!()
    } else {
        match run_launcher(&graphics)? {
            Some(v) => v,
            None => return Ok(()),
        }
    };

    // Setup VMM screen.
    let mut screen = graphics
        .create_screen()
        .map_err(|e| ApplicationError::CreateScreen(Box::new(e)))?;

    // TODO: Start VMM.
    screen
        .run()
        .map_err(|e| ApplicationError::RunScreen(Box::new(e)))?;

    Ok(())
}

fn load_profiles() -> Result<Vec<Profile>, ApplicationError> {
    // TODO: load profiles from filesystem
    let profiles = vec![Profile::default()];

    Ok(profiles)
}

fn run_launcher(graphics: &impl Graphics) -> Result<Option<VmmArgs>, ApplicationError> {
    // Create window and register callback handlers.
    let win = ui::MainWindow::new().map_err(ApplicationError::CreateMainWindow)?;
    let start = Rc::new(RefCell::new(false));

    win.on_start_vmm({
        let win = win.as_weak();
        let start = start.clone();

        move || {
            win.unwrap().hide().unwrap();
            *start.borrow_mut() = true;
        }
    });

    let profiles = load_profiles()?;

    setup_globals(&win);

    let profiles = ModelRc::new(VecModel::from_iter(
        profiles
            .iter()
            .map(|p| SharedString::from(p.name().to_str().unwrap())),
    ));

    win.set_profiles(profiles);

    let physical_devices = ModelRc::new(VecModel::from_iter(
        graphics
            .physical_devices()
            .iter()
            .map(|p| SharedString::from(p.name())),
    ));

    win.set_devices(physical_devices);

    // Run the window.
    win.run().map_err(ApplicationError::RunMainWindow)?;

    drop(win);

    // Extract GUI states.
    let start = Rc::into_inner(start).unwrap().into_inner();

    if !start {
        return Ok(None);
    }

    todo!()
}

fn get_kernel_path(args: &CliArgs) -> Result<PathBuf, ApplicationError> {
    let kernel_path = if let Some(kernel_path) = args.kernel_path() {
        kernel_path.to_path_buf()
    } else {
        let mut pathbuf = std::env::current_exe().map_err(ApplicationError::GetCurrentExePath)?;
        pathbuf.pop();

        #[cfg(target_os = "windows")]
        {
            pathbuf.push("share");
        }

        #[cfg(not(target_os = "windows"))]
        {
            pathbuf.pop();

            #[cfg(target_os = "macos")]
            {
                pathbuf.push("Resources");
            }
            #[cfg(not(target_os = "macos"))]
            {
                pathbuf.push("share");
            }
        }
        pathbuf.push("obkrnl");

        pathbuf
    };

    Ok(kernel_path)
}

fn display_error(e: impl std::error::Error) {
    use std::fmt::Write;

    // Get full message.
    let mut msg = e.to_string();
    let mut src = e.source();

    while let Some(e) = src {
        write!(&mut msg, " -> {e}").unwrap();
        src = e.source();
    }

    // Show error window.
    let win = ErrorDialog::new().unwrap();

    win.set_message(format!("An unexpected error has occurred: {msg}.").into());
    win.run().unwrap();
}

fn setup_globals<'a, T>(component: &'a T)
where
    ui::Globals<'a>: Global<'a, T>,
{
    let globals = ui::Globals::get(component);

    globals.on_open_url(|url| {
        let url = url.as_str();

        if let Err(_e) = open::that(url) {
            // TODO: Show a modal dialog.
        }
    });
}

/// Encapsulates arguments for [`Vmm::new()`].
struct VmmArgs {}

/// Represents an error when [`run()`] fails.
#[derive(Debug, Error)]
enum ApplicationError {
    #[error(transparent)]
    ParseArgs(clap::Error),

    #[cfg(unix)]
    #[error("couldn't increase file descriptor limit")]
    FdLimit(#[source] self::rlim::RlimitError),

    #[error("couldn't run setup wizard")]
    Setup(#[source] SetupError),

    #[error("get current executable path")]
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
    InitGraphics(#[source] Box<dyn std::error::Error>),

    #[error("failed to run main window")]
    RunMainWindow(#[source] slint::PlatformError),

    #[error("couldn't create VMM screen")]
    CreateScreen(#[source] Box<dyn std::error::Error>),

    #[error("couldn't run VMM screen")]
    RunScreen(#[source] Box<dyn std::error::Error>),
}
