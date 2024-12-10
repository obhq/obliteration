use crate::ui::ErrorWindow;
use erdp::ErrorDisplay;
use serde::{Deserialize, Serialize};
use slint::ComponentHandle;
use std::borrow::Cow;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::Mutex;

pub fn spawn_handler(exe: &Path) -> Result<(), std::io::Error> {
    // Spawn the process in panic handler mode.
    let ph = Command::new(exe)
        .args(["--mode", "panic-handler"])
        .stdin(Stdio::piped())
        .spawn()?;

    // Set panic hook to send panic to the handler.
    let ph = Mutex::new(Some(PanicHandler(ph)));

    std::panic::set_hook(Box::new(move |i| panic_hook(i, &ph)));

    Ok(())
}

pub fn run_handler() -> ExitCode {
    use std::io::ErrorKind;

    // Wait for panic info.
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let (msg, exit) = match ciborium::from_reader::<PanicInfo, _>(&mut stdin) {
        Ok(v) => {
            let m = format!(
                "An unexpected error has occurred at {}:{}: {}.",
                v.file, v.line, v.message
            );

            (m, ExitCode::SUCCESS)
        }
        Err(ciborium::de::Error::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            let m = format!("Failed to read panic info: {}.", e.display());

            (m, ExitCode::FAILURE)
        }
    };

    // Display error window.
    let win = ErrorWindow::new().unwrap();

    win.set_message(msg.into());
    win.on_close({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.run().unwrap();

    exit
}

fn panic_hook(i: &PanicHookInfo, ph: &Mutex<Option<PanicHandler>>) {
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
