use slint::{ComponentHandle, ModelExt, ModelRc, SharedString, VecModel};
use args::CliArgs;
use clap::Parser;
use debug::DebugServer;
use std::process::{ExitCode, Termination};
use thiserror::Error;

mod args;
mod debug;
mod error;
mod param;
mod pkg;
mod profile;
#[cfg(unix)]
mod rlim;
mod screen;
mod ui;
mod string;
mod system;
mod vmm;

fn main() -> AppExit {
    let res = run();

    AppExit::from(res)
}

fn run() -> Result<(), ApplicationError> {
    #[cfg(unix)]
    rlim::set_rlimit_nofile();

    let args = CliArgs::try_parse()?;

    if let Some(debug_addr) = args.debug_addr() {
        let debug_server = DebugServer::new(debug_addr)
            .map_err(|e| ApplicationError::StartDebugServer(e, debug_addr))?;

        let debug_client = debug_server
            .accept()
            .map_err(ApplicationError::CreateDebugClient)?;

        //let vmm = Vmm::new(kernel_path, screen, profile, Some(debug_client), event, cx)
    }

    let app = App::new()?;

    app.run()?;

    Ok(())
}

struct App {
    main_window: ui::MainWindow,

    games: ModelRc<ui::Game>,
    profiles: ModelRc<SharedString>,
}

impl App {
    fn new() -> Result<Self, ApplicationError> {
        let main_window = ui::MainWindow::new().map_err(ApplicationError::CreateMainWindow)?;

        let games = ModelRc::new(VecModel::from(Vec::new()));

        main_window.set_games(games.clone());

        let profiles = ModelRc::new(
            VecModel::from(vec![profile::Profile::default()])
                .map(|p| SharedString::from(String::from(p.name().to_string_lossy()))),
        );

        main_window.set_profiles(profiles.clone());

        main_window.on_start_game(|index| {
            let Ok(screen) = ui::Screen::new() else {
                eprintln!("failed to create screen");
                return;
            };

            screen.show().unwrap();
        });

        Ok(Self {
            main_window,
            games,
            profiles,
        })
    }

    fn run(&self) -> Result<(), ApplicationError> {
        self.main_window
            .run()
            .map_err(ApplicationError::RunMainWindow)
    }
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
                eprintln!("{}", e);
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
    ParseArgs(#[from] clap::Error),

    #[error("failed to start debug server on {1} -> {0}")]
    StartDebugServer(
        #[source] debug::StartDebugServerError,
        std::net::SocketAddrV4,
    ),

    #[error("failed to accept debug client -> {0}")]
    CreateDebugClient(#[source] std::io::Error),

    #[error("failed to create main window -> {0}")]
    CreateMainWindow(#[source] ::slint::PlatformError),

    #[error("failed to run main window -> {0}")]
    RunMainWindow(#[source] ::slint::PlatformError),
}
