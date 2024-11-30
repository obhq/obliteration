use crate::dialogs::{open_file, FileType};
use crate::ui::SetupWizard;
use slint::{ComponentHandle, PlatformError};
use std::cell::Cell;
use std::rc::Rc;
use thiserror::Error;

pub fn run_setup() -> Result<bool, SetupError> {
    // TODO: Check if already configured and skip wizard.
    let win = SetupWizard::new().map_err(SetupError::CreateWindow)?;
    let cancel = Rc::new(Cell::new(false));

    win.on_cancel({
        let win = win.as_weak();
        let cancel = cancel.clone();

        move || {
            win.unwrap().hide().unwrap();
            cancel.set(true);
        }
    });

    win.on_browse_firmware({
        let win = win.as_weak();

        move || {
            slint::spawn_local(browse_firmware(win.unwrap())).unwrap();
        }
    });

    // Run the window.
    win.run().map_err(SetupError::RunWindow)?;

    drop(win);

    // Extract GUI states.
    let cancel = Rc::into_inner(cancel).unwrap().into_inner();

    Ok(!cancel)
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

/// Represents an error when [`run_setup()`] fails.
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("couldn't create window")]
    CreateWindow(#[source] PlatformError),

    #[error("couldn't run window")]
    RunWindow(#[source] PlatformError),
}
