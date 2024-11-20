use self::profile::Profile;
use self::ui::ErrorDialog;
use self::vmm::{Vmm, VmmEvent};
use args::CliArgs;
use clap::Parser;
use debug::DebugServer;
use graphics::{GraphicsApi, PhysicalDevice};
use slint::{ComponentHandle, Global, ModelRc, SharedString, VecModel};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use thiserror::Error;

mod args;
mod debug;
mod error;
mod graphics;
mod hv;
mod param;
mod pkg;
mod profile;
#[cfg(unix)]
mod rlim;
mod screen;
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

    // TODO: check if already configured and skip wizard
    run_wizard().map_err(ApplicationError::RunWizard)?;

    if let Some(debug_addr) = args.debug_addr() {
        let kernel_path = get_kernel_path(&args)?;

        let debug_server = DebugServer::new(debug_addr)
            .map_err(|e| ApplicationError::StartDebugServer(e, debug_addr))?;

        let debug_client = debug_server
            .accept()
            .map_err(ApplicationError::CreateDebugClient)?;

        let _graphics_api =
            graphics::DefaultApi::new().map_err(ApplicationError::InitGraphicsApi)?;

        let screen = ui::Screen::new().map_err(ApplicationError::CreateScreen)?;

        let profiles = load_profiles()?;

        // TODO: handle events
        let event_handler = |event| match event {
            VmmEvent::Breakpoint { stop } => {}
            VmmEvent::Log { ty, data, len } => {}
            VmmEvent::Exiting { success } => {}
            VmmEvent::Error { reason } => {}
        };

        let vmm = Vmm::new(
            kernel_path,
            todo!(),
            &profiles[0],
            Some(debug_client),
            event_handler,
        )
        .map_err(ApplicationError::RunVmm)?;
    }

    run_main_app()?;

    Ok(())
}

fn load_profiles() -> Result<Vec<Profile>, ApplicationError> {
    // TODO: load profiles from filesystem
    let profiles = vec![Profile::default()];

    Ok(profiles)
}

fn run_main_app() -> Result<(), ApplicationError> {
    let main_window = ui::MainWindow::new().map_err(ApplicationError::CreateMainWindow)?;

    let graphics_api = graphics::DefaultApi::new().map_err(ApplicationError::InitGraphicsApi)?;

    let profiles = load_profiles()?;

    setup_globals(&main_window);

    let profiles = ModelRc::new(VecModel::from_iter(
        profiles
            .iter()
            .map(|p| SharedString::from(p.name().to_str().unwrap())),
    ));

    main_window.set_profiles(profiles);

    let physical_devices = ModelRc::new(VecModel::from_iter(
        graphics_api
            .physical_devices()
            .iter()
            .map(|p| SharedString::from(p.name())),
    ));

    main_window.set_devices(physical_devices);

    main_window.on_start_game(|_index| {
        // TODO: reuse the same window if possible
        let screen = ui::Screen::new().unwrap();

        screen.show().unwrap();
    });

    main_window.run().map_err(ApplicationError::RunMainWindow)?;

    Ok(())
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

    globals.on_select_file(|title, filter_name, filter| {
        let dialog = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filter_name, &[filter]);

        let path = dialog
            .pick_file()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_default();

        SharedString::from(path)
    });

    globals.on_open_url(|url| {
        let url = url.as_str();

        if let Err(_e) = open::that(url) {
            // TODO: Show a modal dialog.
        }
    });
}

fn run_wizard() -> Result<(), slint::PlatformError> {
    use ui::FileValidationResult;

    ui::Wizard::new().and_then(|wizard| {
        setup_globals(&wizard);

        let wizard_weak = wizard.as_weak();

        wizard.on_cancel(move || {
            wizard_weak.upgrade().inspect(|w| w.hide().unwrap());
        });

        wizard.on_validate_firmware_path(|path| {
            let path: &Path = path.as_str().as_ref();

            if !path.is_absolute() {
                return FileValidationResult::NotAbsolutePath;
            }

            let Ok(metadata) = path.metadata() else {
                return FileValidationResult::DoesNotExist;
            };

            if !metadata.is_file() {
                FileValidationResult::NotFile
            } else {
                FileValidationResult::Ok
            }
        });

        wizard.run()
    })
}

#[derive(Debug, Error)]
enum ApplicationError {
    #[error(transparent)]
    ParseArgs(clap::Error),

    #[cfg(unix)]
    #[error("couldn't increase file descriptor limit")]
    FdLimit(#[source] self::rlim::RlimitError),

    #[error("failed to run wizard")]
    RunWizard(#[source] slint::PlatformError),

    #[error("get current executable path")]
    GetCurrentExePath(#[source] std::io::Error),

    #[error("failed to start debug server on {1}")]
    StartDebugServer(
        #[source] debug::StartDebugServerError,
        std::net::SocketAddrV4,
    ),

    #[error("failed to accept debug connection")]
    CreateDebugClient(#[source] std::io::Error),

    #[error("failed to create screen")]
    CreateScreen(#[source] slint::PlatformError),

    #[error("failed to create main window")]
    CreateMainWindow(#[source] slint::PlatformError),

    #[error("failed to initialize graphics API")]
    InitGraphicsApi(#[source] <graphics::DefaultApi as GraphicsApi>::CreateError),

    #[error("failed to run vmm")]
    RunVmm(#[source] vmm::VmmError),

    #[error("failed to run main window")]
    RunMainWindow(#[source] slint::PlatformError),
}
