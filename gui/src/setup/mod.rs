pub use self::data::DataRootError;

use self::data::{read_data_root, write_data_root};
use crate::data::{DataError, DataMgr};
use crate::dialogs::{open_dir, open_file, FileType};
use crate::ui::SetupWizard;
use erdp::ErrorDisplay;
use obfw::ps4::{PartData, PartReader};
use obfw::{DumpReader, ItemReader};
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

    if let Some(p) = root.as_ref().map(Path::new).filter(|p| p.is_dir()) {
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
        win.set_data_root(v.into());
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

    // Allow only valid unicode path.
    let path = match path.into_os_string().into_string() {
        Ok(v) => v,
        Err(_) => {
            win.set_error_message("Path to selected directory must be unicode.".into());
            return;
        }
    };

    // Set path.
    win.set_data_root(path.into());
}

fn set_data_root(win: SetupWizard) {
    // Get user input.
    let input = win.get_data_root();

    if input.is_empty() {
        win.set_error_message("You need to choose where to store data before proceed.".into());
        return;
    }

    // Check if absolute path.
    let path = Path::new(input.as_str());

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
    if let Err(e) = write_data_root(input) {
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

    // Allow only valid unicode path.
    let path = match path.into_os_string().into_string() {
        Ok(v) => v,
        Err(_) => {
            win.set_error_message("Path to a firmware dump must be unicode.".into());
            return;
        }
    };

    // Set path.
    win.set_firmware_dump(path.into());
}

fn install_firmware(win: SetupWizard) {
    // Get dump path.
    let path = win.get_firmware_dump();

    if path.is_empty() {
        win.set_error_message("You need to select a firmware dump before proceed.".into());
        return;
    }

    // Open firmware dump.
    let mut dump = match File::open(path.as_str())
        .map_err::<Box<dyn Error>, _>(|e| e.into())
        .and_then(|f| DumpReader::new(f).map_err(|e| e.into()))
    {
        Ok(v) => v,
        Err(e) => {
            win.set_error_message(format!("Failed to open {}: {}.", path, e.display()).into());
            return;
        }
    };

    // Create data manager to see if path is writable.
    let root = win.get_data_root();
    let dmgr = match DataMgr::new(root.as_str()) {
        Ok(v) => v,
        Err(e) => {
            let m = format!(
                "Failed to create data manager on {}: {}.",
                root,
                e.display()
            );

            win.set_error_message(m.into());
            return;
        }
    };

    win.invoke_show_firmware_installer();
    win.set_firmware_status("Initializing...".into());

    // Spawn thread to extract the dump.
    let win = win.as_weak();

    std::thread::spawn(move || {
        // Extract.
        let n = dump.items();
        let mut p = 0u32;
        let e =
            match extract_firmware_dump(
                &mut dump,
                &dmgr,
                |v| drop(win.upgrade_in_event_loop(move |w| w.set_firmware_status(v.into()))),
                || {
                    p += 1;

                    drop(win.upgrade_in_event_loop(move |w| {
                        w.set_firmware_progress(p as f32 / n as f32)
                    }));
                },
            ) {
                Ok(_) => {
                    drop(win.upgrade_in_event_loop(|w| w.invoke_set_firmware_finished(true)));
                    return;
                }
                Err(e) => e,
            };

        // Show error.
        let m = format!("Failed to install {}: {}.", path, e.display());

        drop(win.upgrade_in_event_loop(move |w| {
            w.invoke_set_firmware_finished(false);
            w.set_error_message(m.into());
        }));
    });
}

fn extract_firmware_dump(
    dump: &mut DumpReader<File>,
    dmgr: &DataMgr,
    mut status: impl FnMut(String),
    mut step: impl FnMut(),
) -> Result<(), FirmwareError> {
    loop {
        // Get next item.
        let mut item = match dump.next_item().map_err(FirmwareError::NextItem)? {
            Some(v) => v,
            None => break,
        };

        // Update status.
        let name = item.to_string();

        status(format!("Extracting {name}..."));

        // Extract item.
        let r: Result<(), Box<dyn Error>> = match &mut item {
            ItemReader::Ps4Part(r) => {
                extract_partition(dmgr, r, &mut status, &mut step).map_err(|e| e.into())
            }
        };

        if let Err(e) = r {
            return Err(FirmwareError::ExtractItem(name, e));
        }

        step();
    }

    Ok(())
}

fn extract_partition(
    dmgr: &DataMgr,
    part: &mut PartReader<File>,
    status: &mut impl FnMut(String),
    step: &mut impl FnMut(),
) -> Result<(), PartitionError> {
    // Get device path.
    let dev = part.dev();
    let dev = match std::str::from_utf8(dev) {
        Ok(v) => v,
        Err(_) => {
            let n = String::from_utf8_lossy(dev);
            return Err(PartitionError::UnexpectedDevice(n.into_owned()));
        }
    };

    // Get device name.
    let dev = if dev == "md0" {
        dev
    } else if let Some(v) = dev.strip_prefix("/dev/") {
        if v.contains(['/', '\\']) {
            return Err(PartitionError::UnexpectedDevice(dev.into()));
        }

        v
    } else {
        return Err(PartitionError::UnexpectedDevice(dev.into()));
    };

    // Extract items.
    let root = dmgr.part().data(dev);

    loop {
        // Get next item.
        let item = match part.next_item().map_err(PartitionError::NextItem)? {
            Some(v) => v,
            None => break,
        };

        // Unpack item.
        let (name, data) = match item {
            PartData::Directory(n) => (n, None),
            PartData::File(n, r) => (n, Some(r)),
        };

        // Get name.
        let name = match String::from_utf8(name) {
            Ok(v) => v,
            Err(e) => {
                let n = String::from_utf8_lossy(e.as_bytes());
                return Err(PartitionError::UnexpectedFile(n.into_owned()));
            }
        };

        // Get local path.
        let mut path = root.to_path_buf();

        for com in name.split('/').skip(1) {
            if com.is_empty() || com.contains('\\') {
                return Err(PartitionError::UnexpectedFile(name));
            }

            path.push(com);
        }

        // Extract item.
        match data {
            Some(mut data) => {
                status(format!("Extracting {name}..."));

                // Create only if not exists.
                let mut file = match File::create_new(&path) {
                    Ok(v) => v,
                    Err(e) => return Err(PartitionError::CreateFile(path, e)),
                };

                if let Err(e) = std::io::copy(&mut data, &mut file) {
                    return Err(PartitionError::ExtractFile(name, path, e));
                }
            }
            None => {
                // Create only if not exists.
                if let Err(e) = std::fs::create_dir(&path) {
                    return Err(PartitionError::CreateDirectory(path, e));
                }
            }
        }

        step();
    }

    Ok(())
}

/// Represents an error when [`extract_firmware_dump()`] fails.
#[derive(Debug, Error)]
enum FirmwareError {
    #[error("couldn't get dumped item")]
    NextItem(#[source] obfw::ReaderError),

    #[error("couldn't extract {0}")]
    ExtractItem(String, #[source] Box<dyn Error>),
}

/// Represents an error when [`extract_partition()`] fails.
#[derive(Debug, Error)]
enum PartitionError {
    #[error("unexpected device {0}")]
    UnexpectedDevice(String),

    #[error("couldn't get partition item")]
    NextItem(#[source] obfw::ps4::PartError),

    #[error("unexpected file {0}")]
    UnexpectedFile(String),

    #[error("couldn't create {0}")]
    CreateDirectory(PathBuf, #[source] std::io::Error),

    #[error("couldn't create {0}")]
    CreateFile(PathBuf, #[source] std::io::Error),

    #[error("couldn't extract {0} to {1}")]
    ExtractFile(String, PathBuf, #[source] std::io::Error),
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
