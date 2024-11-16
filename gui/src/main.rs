use args::CliArgs;
use clap::Parser;
use debug::DebugServer;
use graphics::{GraphicsApi, PhysicalDevice};
use slint::{ComponentHandle, ModelExt, ModelRc, SharedString, VecModel};
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
    let res = run();

    AppExit::from(res)
}

fn run() -> Result<(), ApplicationError> {
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

    // TODO: check if already configured an skip wizard
    run_wizard().map_err(ApplicationError::RunWizard)?;

    let args = CliArgs::try_parse().map_err(ApplicationError::ParseArgs)?;

    if let Some(debug_addr) = args.debug_addr() {
        let debug_server = DebugServer::new(debug_addr)
            .map_err(|e| ApplicationError::StartDebugServer(e, debug_addr))?;

        let debug_client = debug_server
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

fn run_wizard() -> Result<(), slint::PlatformError> {
    use ui::FileValidationResult;

    ui::Wizard::new().and_then(|wizard| {
        wizard.on_validate_system_dir(|path: SharedString| {
            let path = AsRef::<Path>::as_ref(path.as_str());

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
            let system_path = AsRef::<Path>::as_ref(system_path.as_str());
            let games_path = AsRef::<Path>::as_ref(games_path.as_str());

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
            AppExit::Err(e) => {
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

                ExitCode::FAILURE
            }
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
