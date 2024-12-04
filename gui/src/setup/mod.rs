pub use self::data::DataRootError;

use self::data::{read_data_root, write_data_root};
use crate::data::{DataError, DataMgr};
use crate::dialogs::{open_dir, open_file, FileType};
use crate::ui::SetupWizard;
use erdp::ErrorDisplay;
use obfw::DumpReader;
use slint::{ComponentHandle, PlatformError};
use std::cell::Cell;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod data;

pub fn run_setup() -> Result<Option<DataMgr>, SetupError> {
    // Load data root.
    let root = read_data_root().map_err(SetupError::ReadDataRoot)?;

    if let Some(p) = root.as_ref().filter(|p| p.is_dir()) {
        // Check if root partition exists.
        let mgr = DataMgr::new(p).map_err(|e| SetupError::DataManager(p.to_owned(), e))?;

        if mgr.part().meta("md0").is_file() {
            return Ok(Some(mgr));
        }
    }

    // Create setup wizard.
    let win = SetupWizard::new().map_err(SetupError::CreateWindow)?;
    let finish = Rc::new(Cell::new(false));

    win.on_cancel({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.on_browse_data_root({
        let win = win.as_weak();

        move || {
            slint::spawn_local(browse_data_root(win.unwrap())).unwrap();
        }
    });

    win.on_set_data_root({
        let win = win.as_weak();

        move || set_data_root(win.unwrap())
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

    if let Some(v) = root {
        win.set_data_root(v.into_os_string().into_string().unwrap().into());
    }

    // Run the wizard.
    win.run().map_err(SetupError::RunWindow)?;

    drop(win);

    // Check how the user exit the wizard.
    let finish = Rc::into_inner(finish).unwrap().into_inner();

    if !finish {
        return Ok(None);
    }

    todo!()
}

async fn browse_data_root(win: SetupWizard) {
    // Ask the user to browse for a directory.
    let path = match open_dir(&win, "Data location").await {
        Some(v) => v,
        None => return,
    };

    // Set path.
    win.set_data_root(path.into_os_string().into_string().unwrap().into());
}

fn set_data_root(win: SetupWizard) {
    // Get path.
    let path = win.get_data_root();

    if path.is_empty() {
        win.set_error_message("You need to choose where to store data before proceed.".into());
        return;
    }

    // Check if absolute path.
    let path = Path::new(path.as_str());

    if !path.is_absolute() {
        win.set_error_message("Path must be absolute.".into());
        return;
    } else if !path.is_dir() {
        win.set_error_message("Path must be a directory.".into());
        return;
    }

    // Create data manager to see if path is writable.
    let mgr = match DataMgr::new(path) {
        Ok(v) => v,
        Err(e) => {
            win.set_error_message(
                format!("Failed to create data manager: {}.", e.display()).into(),
            );
            return;
        }
    };

    // Save.
    if let Err(e) = write_data_root(path) {
        win.set_error_message(format!("Failed to save data location: {}.", e.display()).into());
        return;
    }

    win.invoke_set_data_root_ok(mgr.part().meta("md0").is_file());
}

async fn browse_firmware(win: SetupWizard) {
    // Ask the user to browse for a file.
    let path = match open_file(&win, "Select a firmware dump", FileType::Firmware).await {
        Some(v) => v,
        None => return,
    };

    // Set path.
    win.set_firmware_dump(path.into_os_string().into_string().unwrap().into());
}

fn install_firmware(win: SetupWizard) {
    // Get dump path.
    let dump = win.get_firmware_dump();

    if dump.is_empty() {
        win.set_error_message("You need to select a firmware dump before proceed.".into());
        return;
    }

    // Open firmware dump.
    let dump = match File::open(dump.as_str())
        .map_err::<Box<dyn Error>, _>(|e| e.into())
        .and_then(|f| DumpReader::new(f).map_err(|e| e.into()))
    {
        Ok(v) => v,
        Err(e) => {
            win.set_error_message(format!("Failed to open {}: {}.", dump, e.display()).into());
            return;
        }
    };

    // TODO: Spawn a thread to extract the dump.
    win.invoke_show_firmware_installer();
}

/// Represents an error when [`run_setup()`] fails.
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("couldn't read data location")]
    ReadDataRoot(#[source] DataRootError),

    #[error("couldn't create data manager on {0}")]
    DataManager(PathBuf, #[source] DataError),

    #[error("couldn't create setup wizard")]
    CreateWindow(#[source] PlatformError),

    #[error("couldn't run setup wizard")]
    RunWindow(#[source] PlatformError),
}
