use crate::ui::{App, ErrorWindow, RuntimeExt};
use erdp::ErrorDisplay;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, SharedString};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::panic::PanicHookInfo;
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::Mutex;
use thiserror::Error;

pub fn spawn_handler(exe: &Path) -> Result<(), std::io::Error> {
    // Spawn the process in panic handler mode.
    let ph = Command::new(exe)
        .args(["--mode", "panic-handler"])
        .stdin(Stdio::piped())
        .spawn()?;

    // Set panic hook to send panic to the handler.
    let ph = Mutex::new(Some(HandlerProcess(ph)));

    std::panic::set_hook(Box::new(move |i| panic_hook(i, &ph)));

    Ok(())
}

pub fn run_handler() -> ExitCode {
    // Wait for panic info.
    let mut buf = Vec::new();
    let (msg, exit) = match std::io::stdin().read_to_end(&mut buf) {
        Ok(0) => return ExitCode::SUCCESS,
        Ok(_) => match minicbor_serde::from_slice::<PanicInfo>(&buf) {
            Ok(v) => {
                let m = slint::format!(
                    "An unexpected error has occurred at {}:{}: {}.",
                    v.file,
                    v.line,
                    v.message
                );

                (m, ExitCode::SUCCESS)
            }
            Err(e) => {
                let m = slint::format!("Failed to decode panic info: {}.", e.display());

                (m, ExitCode::FAILURE)
            }
        },
        Err(e) => {
            let m = slint::format!("Failed to read panic info: {}.", e.display());

            (m, ExitCode::FAILURE)
        }
    };

    match crate::ui::run::<PanicHandler>(msg) {
        ExitCode::SUCCESS => exit,
        v => v,
    }
}

fn panic_hook(i: &PanicHookInfo, ph: &Mutex<Option<HandlerProcess>>) {
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
        message: Cow::Borrowed(i.payload_as_str().unwrap_or("unknown panic payload")),
        file: loc.file().into(),
        line: loc.line(),
    };

    stdin
        .write_all(&minicbor_serde::to_vec(info).unwrap())
        .unwrap();
    stdin.flush().unwrap();

    drop(stdin); // Close the stdin to notify panic handler that no more data.
}

/// Provide [`Drop`] implementation to shutdown panic handler.
struct HandlerProcess(Child);

impl Drop for HandlerProcess {
    fn drop(&mut self) {
        // wait() will close stdin for us before waiting.
        self.0.wait().unwrap();
    }
}

/// Implementation of [`App`] for panic handler process.
struct PanicHandler {
    msg: SharedString,
}

impl App for PanicHandler {
    type Err = PanicError;
    type Args = SharedString;

    const NAME: &str = "panic handler";

    fn new(args: Self::Args) -> Result<Self, Self::Err> {
        Ok(Self { msg: args })
    }

    async fn run(self) -> Result<(), Self::Err> {
        // Setup error window.
        let win = ErrorWindow::new().map_err(PanicError::CreateErrorWindow)?;

        win.set_message(self.msg);
        win.on_close({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        // Run the window.
        win.show().map_err(PanicError::ShowErrorWindow)?;
        win.wait().await;

        Ok(())
    }
}

/// Contains panic information from the VMM process.
#[derive(Serialize, Deserialize)]
struct PanicInfo<'a> {
    message: Cow<'a, str>,
    file: Cow<'a, str>,
    line: u32,
}

/// Represents an error when [`PanicHandler`] fails.
#[derive(Debug, Error)]
enum PanicError {
    #[error("couldn't create error window")]
    CreateErrorWindow(#[source] slint::PlatformError),

    #[error("couldn't show error window")]
    ShowErrorWindow(#[source] slint::PlatformError),
}
