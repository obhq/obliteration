use args::CliArgs;
use clap::Parser;
use debug::DebugServer;
use graphics::{GraphicsApi, PhysicalDevice};
use slint::{ComponentHandle, Global, ModelExt, ModelRc, SharedString, VecModel};
use std::path::Path;
use std::process::{ExitCode, Termination};
use thiserror::Error;

mod args;
mod debug;
mod error;
mod graphics;
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

fn main() -> AppExit {
    let res = run().inspect_err(|e| {
        ui::ErrorDialog::new()
            .and_then(|error_dialog| {
                error_dialog.set_message(SharedString::from(format!(
                    "Error running application: {}",
                    full_error_reason(e)
                )));

                error_dialog.run()
            })
            .inspect_err(|e| eprintln!("Error displaying error dialog: {e}"))
            .unwrap();
    });

    AppExit::from(res)
}

fn run() -> Result<(), ApplicationError> {
    let args = CliArgs::try_parse().map_err(ApplicationError::ParseArgs)?;

    #[cfg(unix)]
    if let Err(e) = rlim::set_rlimit_nofile() {
        ui::ErrorDialog::new()
            .and_then(|error_dialog| {
                error_dialog.set_message(SharedString::from(format!(
                    "Error setting rlimit: {}",
                    full_error_reason(e)
                )));

                error_dialog.run()
            })
            .inspect_err(|e| eprintln!("Error displaying error dialog: {e}"))
            .unwrap();
    }

    // TODO: check if already configured and skip wizard
    run_wizard().map_err(ApplicationError::RunWizard)?;

    if let Some(debug_addr) = args.debug_addr() {
        let debug_server = DebugServer::new(debug_addr)
            .map_err(|e| ApplicationError::StartDebugServer(e, debug_addr))?;

        let _debug_client = debug_server
            .accept()
            .map_err(ApplicationError::CreateDebugClient)?;
    }

    run_main_app()?;

    Ok(())
}

fn run_main_app() -> Result<(), ApplicationError> {
    let main_window = ui::MainWindow::new().map_err(ApplicationError::CreateMainWindow)?;

    let graphics_api = graphics::DefaultApi::new().map_err(ApplicationError::InitGraphicsApi)?;

    let devices: Vec<SharedString> = graphics_api
        .physical_devices()
        .into_iter()
        .map(|d| SharedString::from(d.name()))
        .collect();

    main_window.set_devices(ModelRc::new(VecModel::from(devices)));

    let profiles = ModelRc::new(
        VecModel::from(vec![profile::Profile::default()])
            .map(|p| SharedString::from(String::from(p.name().to_string_lossy()))),
    );

    main_window.set_profiles(profiles.clone());

    main_window.on_start_game(|_index| {
        let screen = ui::Screen::new().unwrap();

        screen.show().unwrap();
    });

    main_window.run().map_err(ApplicationError::RunMainWindow)?;

    Ok(())
}

fn setup_global_callbacks<'a, T>(component: &'a T)
where
    ui::GlobalCallbacks<'a>: Global<'a, T>,
{
    let global_callbacks = ui::GlobalCallbacks::get(component);

    global_callbacks.on_select_folder(|title| {
        let dialog = rfd::FileDialog::new().set_title(title);

        let path = dialog
            .pick_file()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_default();

        SharedString::from(path)
    });

    global_callbacks.on_select_file(|title, filter_name, filter| {
        let dialog = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filter_name, &[filter]);

        let path = dialog
            .pick_file()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_default();

        SharedString::from(path)
    });
}

fn run_wizard() -> Result<(), slint::PlatformError> {
    use ui::FileValidationResult;

    ui::Wizard::new().and_then(|wizard| {
        setup_global_callbacks(&wizard);

        let wizard_weak = wizard.as_weak();

        wizard.on_cancel(move || {
            wizard_weak.upgrade().inspect(|w| w.hide().unwrap());
        });

        wizard.on_validate_system_dir(|path| {
            let path: &Path = path.as_str().as_ref();

            if !path.is_absolute() {
                return FileValidationResult::NotAbsolutePath;
            }

            let Ok(metadata) = path.metadata() else {
                return FileValidationResult::DoesNotExist;
            };

            if !metadata.is_dir() {
                FileValidationResult::NotDirectory
            } else {
                FileValidationResult::Ok
            }
        });

        wizard.on_validate_games_dir(|system_path, games_path| {
            let system_path: &Path = system_path.as_str().as_ref();
            let games_path: &Path = games_path.as_str().as_ref();

            if !games_path.is_absolute() {
                return FileValidationResult::NotAbsolutePath;
            }

            let Ok(metadata) = games_path.metadata() else {
                return FileValidationResult::DoesNotExist;
            };

            if !metadata.is_dir() {
                FileValidationResult::NotDirectory
            } else if games_path == system_path {
                FileValidationResult::SameAsSystemDir
            } else {
                FileValidationResult::Ok
            }
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

fn full_error_reason<T>(e: T) -> String
where
    T: std::error::Error,
{
    use std::fmt::Write;

    let mut msg = format!("{e}");
    let mut src = e.source();

    while let Some(e) = src {
        write!(&mut msg, " -> {e}").unwrap();
        src = e.source();
    }

    msg
}

pub enum AppExit {
    Ok,
    Err(ApplicationError),
}

impl Termination for AppExit {
    fn report(self) -> ExitCode {
        match self {
            AppExit::Ok => ExitCode::SUCCESS,
            AppExit::Err(e) => ExitCode::FAILURE,
        }
    }
}

impl From<Result<(), ApplicationError>> for AppExit {
    fn from(v: Result<(), ApplicationError>) -> Self {
        match v {
            Ok(_) => AppExit::Ok,
            Err(e) => AppExit::Err(e),
        }
    }
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    ParseArgs(clap::Error),

    #[error("failed to run wizard")]
    RunWizard(#[source] slint::PlatformError),

    #[error("failed to start debug server on {1}")]
    StartDebugServer(
        #[source] debug::StartDebugServerError,
        std::net::SocketAddrV4,
    ),

    #[error("failed to accept debug client")]
    CreateDebugClient(#[source] std::io::Error),

    #[error("failed to create main window")]
    CreateMainWindow(#[source] slint::PlatformError),

    #[error("failed to initialize graphics API")]
    InitGraphicsApi(#[source] <graphics::DefaultApi as GraphicsApi>::CreateError),

    #[error("failed to run main window")]
    RunMainWindow(#[source] slint::PlatformError),
}
