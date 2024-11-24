use crate::dialogs::{open_file, FileType};
use crate::ui::SetupWizard;
use slint::{ComponentHandle, PlatformError};
use thiserror::Error;

pub fn run_setup() -> Result<(), SetupError> {
    // TODO: Check if already configured and skip wizard.
    let win = SetupWizard::new().map_err(SetupError::CreateWindow)?;

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

    win.run().map_err(SetupError::RunWindow)?;

    Ok(())
}

async fn browse_firmware(win: SetupWizard) {
    open_file(&win, "Select a firmware dump", FileType::Firmware).await;
}

/// Represents an error when [`run_setup()`] fails.
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("couldn't create window")]
    CreateWindow(#[source] PlatformError),

    #[error("couldn't run window")]
    RunWindow(#[source] PlatformError),
}
