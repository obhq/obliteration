use crate::dialogs::{open_file, FileType};
use crate::ui::SetupWizard;
use erdp::ErrorDisplay;
use slint::{ComponentHandle, PlatformError};
use std::cell::Cell;
use std::fs::File;
use std::rc::Rc;
use thiserror::Error;

pub fn run_setup() -> Result<bool, SetupError> {
    // TODO: Check if already configured and skip wizard.
    let win = SetupWizard::new().map_err(SetupError::CreateWindow)?;
    let finish = Rc::new(Cell::new(false));

    win.on_cancel({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.on_browse_firmware({
        let win = win.as_weak();

        move || {
            slint::spawn_local(browse_firmware(win.unwrap())).unwrap();
        }
    });

    win.on_install_firmware({
        let win = win.as_weak();

        move || install_firmware(win.unwrap())
    });

    win.on_finish({
        let win = win.as_weak();
        let finish = finish.clone();

        move || {
            win.unwrap().hide().unwrap();
            finish.set(true);
        }
    });

    // Run the window.
    win.run().map_err(SetupError::RunWindow)?;

    drop(win);

    // Extract GUI states.
    let finish = Rc::into_inner(finish).unwrap().into_inner();

    Ok(finish)
}

async fn browse_firmware(win: SetupWizard) {
    // Ask the user to browse a file.
    let path = match open_file(&win, "Select a firmware dump", FileType::Firmware).await {
        Some(v) => v,
        None => return,
    };

    // Set path.
    win.set_firmware_dump(path.into_os_string().into_string().unwrap().into());
}

fn install_firmware(win: SetupWizard) {
    // Open firmware dump.
    let dump = win.get_firmware_dump();
    let dump = match File::open(dump.as_str()) {
        Ok(v) => v,
        Err(e) => {
            win.set_error_message(format!("Failed to open {}: {}.", dump, e.display()).into());
            return;
        }
    };
}

/// Represents an error when [`run_setup()`] fails.
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("couldn't create window")]
    CreateWindow(#[source] PlatformError),

    #[error("couldn't run window")]
    RunWindow(#[source] PlatformError),
}
