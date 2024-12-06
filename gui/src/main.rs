#![windows_subsystem = "windows"]

use self::graphics::{Graphics, PhysicalDevice, Screen};
use self::profile::{Profile, ProfileModel};
use self::setup::{run_setup, SetupError};
use self::ui::{ErrorWindow, MainWindow};
use clap::{Parser, ValueEnum};
use debug::DebugServer;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Global, ModelExt, ModelRc, SharedString, VecModel};
use std::borrow::Cow;
use std::cell::Cell;
use std::error::Error;
use std::io::Write;
use std::net::SocketAddrV4;
use std::panic::PanicHookInfo;
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};
use thiserror::Error;

mod data;
mod debug;
mod dialogs;
mod graphics;
mod hv;
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
    let args = Args::parse();
    let r = match &args.mode {
        Some(ProgramMode::PanicHandler) => run_panic_handler(),
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

fn run_vmm(args: &Args) -> Result<(), ApplicationError> {
    // Resolve our executable path.
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(ApplicationError::GetCurrentExePath)?;

    // Spawn panic handler.
    let ph = Command::new(&exe)
        .args(["--mode", "panic-handler"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(ApplicationError::SpawnPanicHandler)?;

    // Set panic handler.
    let ph = Arc::new(Mutex::new(Some(PanicHandler(ph))));
    let ph = Arc::downgrade(&ph);

    std::panic::set_hook(Box::new(move |i| panic_hook(i, &ph)));

    #[cfg(unix)]
    rlim::set_rlimit_nofile().map_err(ApplicationError::FdLimit)?;

    // Initialize graphics engine.
    let mut graphics = match graphics::DefaultApi::new() {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::InitGraphics(Box::new(e))),
    };

    // Run setup wizard. This will simply return the data manager if the user already has required
    // settings.
    let data = match run_setup().map_err(ApplicationError::Setup)? {
        Some(v) => v,
        None => return Ok(()),
    };

    // Get kernel path.
    let kernel = match &args.kernel {
        Some(v) => v.to_path_buf(),
        None => {
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
        }
    };

    // TODO: load profiles from filesystem
    let profiles = vec![Profile::default()];

    // Get VMM arguments.
    let args = if let Some(debug_addr) = args.debug {
        let debug_server = DebugServer::new(debug_addr)
            .map_err(|e| ApplicationError::StartDebugServer(e, debug_addr))?;

        let debug_client = debug_server
            .accept()
            .map_err(ApplicationError::CreateDebugClient)?;

        todo!()
    } else {
        match run_launcher(&graphics, profiles)? {
            Some(v) => v,
            None => return Ok(()),
        }
    };

    // Setup VMM screen.
    let screen = graphics
        .create_screen()
        .map_err(|e| ApplicationError::CreateScreen(Box::new(e)))?;

    // TODO: Start VMM.
    screen
        .run()
        .map_err(|e| ApplicationError::RunScreen(Box::new(e)))?;

    Ok(())
}

fn run_panic_handler() -> Result<(), ApplicationError> {
    use std::io::ErrorKind;

    // Wait for panic info.
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let info: PanicInfo = match ciborium::from_reader(&mut stdin) {
        Ok(v) => v,
        Err(ciborium::de::Error::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => return Ok(()),
        Err(e) => return Err(ApplicationError::ReadPanicInfo(e)),
    };

    // Display panic info.
    let win = ErrorWindow::new().unwrap();
    let msg = format!(
        "An unexpected error has occurred at {}:{}: {}.",
        info.file, info.line, info.message
    );

    win.set_message(msg.into());
    win.on_close({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.run().unwrap();

    Ok(())
}

fn run_launcher(
    graphics: &impl Graphics,
    profiles: Vec<Profile>,
) -> Result<Option<VmmArgs>, ApplicationError> {
    // Create window and register callback handlers.
    let win = MainWindow::new().map_err(ApplicationError::CreateMainWindow)?;
    let profiles = Rc::new(ProfileModel::new(profiles));
    let start = Rc::new(Cell::new(false));

    win.on_profile_names(|profiles| ModelRc::new(profiles.map(|p| p.name)));

    win.on_start_vmm({
        let win = win.as_weak();
        let start = start.clone();

        move || {
            win.unwrap().hide().unwrap();
            start.set(true);
        }
    });

    let physical_devices = ModelRc::new(VecModel::from_iter(
        graphics
            .physical_devices()
            .iter()
            .map(|p| SharedString::from(p.name())),
    ));

    win.set_devices(physical_devices);
    win.set_profiles(profiles.into());

    // Run the window.
    win.run().map_err(ApplicationError::RunMainWindow)?;

    drop(win);

    // Extract GUI states.
    let start = Rc::into_inner(start)
        .unwrap()
        .into_inner()
        .then_some(VmmArgs {});

    Ok(start)
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

fn panic_hook(i: &PanicHookInfo, ph: &Weak<Mutex<Option<PanicHandler>>>) {
    // Check if panic handler still alive.
    let ph = match ph.upgrade() {
        Some(v) => v,
        None => {
            // The only cases for us to be here is we panic after returned from run_vmm().
            eprintln!("{i}");
            return;
        }
    };

    // Allow only one thread to report the panic.
    let mut ph = ph.lock().unwrap();
    let mut ph = match ph.take() {
        Some(v) => v,
        None => {
            // There are another thread already panicked when we are here. The process will be
            // aborted once the previous thread has return from this hook. The only possible cases
            // for the other thread to be here is because the abortion from the previous panic is
            // not finished yet. So better to not print the panic here because it may not work.
            return;
        }
    };

    // Send panic information.
    let mut stdin = ph.0.stdin.take().unwrap();
    let loc = i.location().unwrap();
    let info = PanicInfo {
        message: if let Some(&s) = i.payload().downcast_ref::<&str>() {
            s.into()
        } else if let Some(s) = i.payload().downcast_ref::<String>() {
            s.into()
        } else {
            "unknown panic payload".into()
        },
        file: loc.file().into(),
        line: loc.line(),
    };

    ciborium::into_writer(&info, &mut stdin).unwrap();
    stdin.flush().unwrap();

    drop(stdin); // Close the stdin to notify panic handler that no more data.
}

/// Program arguments parsed from command line.
#[derive(Parser)]
#[command(about = None)]
struct Args {
    #[arg(long, value_enum, hide = true)]
    mode: Option<ProgramMode>,

    /// Immediate launch the VMM in debug mode.
    #[arg(long)]
    debug: Option<SocketAddrV4>,

    /// Use the kernel image at the specified path instead of the default one.
    #[arg(long)]
    kernel: Option<Box<Path>>,
}

/// Encapsulates arguments for [`Vmm::new()`].
struct VmmArgs {}

/// Mode of our program.
#[derive(Clone, ValueEnum)]
enum ProgramMode {
    PanicHandler,
}

/// Provide [`Drop`] implementation to shutdown panic handler.
struct PanicHandler(Child);

impl Drop for PanicHandler {
    fn drop(&mut self) {
        // wait() will close stdin for us before waiting.
        self.0.wait().unwrap();
    }
}

/// Contains panic information from the VMM process.
#[derive(Serialize, Deserialize)]
struct PanicInfo<'a> {
    message: Cow<'a, str>,
    file: Cow<'a, str>,
    line: u32,
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

    #[error("couldn't read panic info")]
    ReadPanicInfo(#[source] ciborium::de::Error<std::io::Error>),
}
