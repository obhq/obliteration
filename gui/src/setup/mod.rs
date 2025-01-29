pub use self::data::DataRootError;

use self::data::{read_data_root, write_data_root};
use crate::data::{DataError, DataMgr};
use crate::ui::{
    error, open_dir, open_file, spawn_handler, DesktopExt, FileType, InstallFirmware, RuntimeExt,
    SetupWizard,
};
use crate::vfs::{FsType, FS_TYPE};
use erdp::ErrorDisplay;
use obfw::ps4::{PartData, PartReader};
use obfw::{DumpReader, ItemReader};
use redb::{Database, DatabaseError};
use slint::{CloseRequestResponse, ComponentHandle, PlatformError, SharedString};
use std::cell::Cell;
use std::error::Error;
use std::fs::File;
use std::future::Future;
use std::io::{ErrorKind, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod data;

pub async fn run_setup() -> Result<Option<DataMgr>, SetupError> {
    // Load data root.
    let root = read_data_root().map_err(SetupError::ReadDataRoot)?;

    if let Some(p) = root.as_ref().map(Path::new).filter(|p| p.is_dir()) {
        // Check if root partition exists.
        let mgr = DataMgr::new(p).map_err(|e| SetupError::DataManager(p.to_owned(), e))?;

        if mgr.partitions().meta("md0").is_file() {
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

    win.on_get_dumper({
        let win = win.as_weak();

        move || spawn_handler(&win, |w| get_dumper(w))
    });

    win.on_browse_data_root({
        let win = win.as_weak();

        move || spawn_handler(&win, |w| browse_data_root(w))
    });

    win.on_set_data_root({
        let win = win.as_weak();

        move || spawn_handler(&win, |w| set_data_root(w))
    });

    win.on_browse_firmware({
        let win = win.as_weak();

        move || spawn_handler(&win, |w| browse_firmware(w))
    });

    win.on_install_firmware({
        let win = win.as_weak();

        move || spawn_handler(&win, |w| install_firmware(w))
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
    win.show().map_err(SetupError::ShowWindow)?;
    win.set_center().map_err(SetupError::CenterWindow)?;
    win.wait().await;

    drop(win);

    // Check how the user exit the wizard.
    let finish = Rc::into_inner(finish).unwrap().into_inner();

    if !finish {
        return Ok(None);
    }

    // Load data root.
    let root = read_data_root().map_err(SetupError::ReadDataRoot)?.unwrap();
    let mgr = match DataMgr::new(root.as_str()) {
        Ok(v) => v,
        Err(e) => return Err(SetupError::DataManager(root.into(), e)),
    };

    Ok(Some(mgr))
}

async fn get_dumper(win: SetupWizard) {
    // Open web browser.
    let url = "https://github.com/obhq/firmware-dumper";
    let e = match open::that_detached(url) {
        Ok(_) => return,
        Err(e) => e,
    };

    // Show error.
    let m = slint::format!("Failed to open {}: {}.", url, e.display());

    error(Some(&win), m).await;
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
            error(Some(&win), "Path to selected directory must be unicode.").await;
            return;
        }
    };

    // Set path.
    win.set_data_root(path.into());
}

async fn set_data_root(win: SetupWizard) {
    // Get user input.
    let input = win.get_data_root();

    if input.is_empty() {
        let msg = SharedString::from("You need to choose where to store data before proceed.");
        error(Some(&win), msg).await;
        return;
    }

    // Check if absolute path.
    let path = Path::new(input.as_str());

    if !path.is_absolute() {
        error(Some(&win), "Path must be absolute.").await;
        return;
    } else if !path.is_dir() {
        error(Some(&win), "Path must be a directory.").await;
        return;
    }

    // Create data manager to see if path is writable.
    let mgr = match DataMgr::new(path) {
        Ok(v) => v,
        Err(e) => {
            let m = slint::format!("Failed to create data manager: {}.", e.display());
            error(Some(&win), m).await;
            return;
        }
    };

    // Save.
    if let Err(e) = write_data_root(input) {
        let m = slint::format!("Failed to save data location: {}.", e.display());
        error(Some(&win), m).await;
        return;
    }

    win.invoke_set_data_root_ok(mgr.partitions().meta("md0").is_file());
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
            error(Some(&win), "Path to a firmware dump must be unicode.").await;
            return;
        }
    };

    // Set path.
    win.set_firmware_dump(path.into());
}

async fn install_firmware(win: SetupWizard) {
    // Get dump path.
    let path = win.get_firmware_dump();

    if path.is_empty() {
        let m = "You need to select a firmware dump before proceed.";
        error(Some(&win), m).await;
        return;
    }

    // Open firmware dump.
    let mut dump = match File::open(path.as_str())
        .map_err::<Box<dyn Error>, _>(|e| e.into())
        .and_then(|f| DumpReader::new(f).map_err(|e| e.into()))
    {
        Ok(v) => v,
        Err(e) => {
            let m = slint::format!("Failed to open {}: {}.", path, e.display());
            error(Some(&win), m).await;
            return;
        }
    };

    // Create data manager to see if path is writable.
    let root = win.get_data_root();
    let dmgr = match DataMgr::new(root.as_str()) {
        Ok(v) => v,
        Err(e) => {
            let m = slint::format!(
                "Failed to create data manager on {}: {}.",
                root,
                e.display()
            );

            error(Some(&win), m).await;
            return;
        }
    };

    // Setup progress window.
    let pw = match InstallFirmware::new() {
        Ok(v) => v,
        Err(e) => {
            let m = slint::format!(
                "Failed to create firmware progress window: {}.",
                e.display()
            );

            error(Some(&win), m).await;
            return;
        }
    };

    pw.set_status("Initializing...".into());
    pw.window()
        .on_close_requested(|| CloseRequestResponse::KeepWindowShown);

    if let Err(e) = pw.show() {
        let m = slint::format!("Failed to show firmware progress window: {}.", e.display());
        error(Some(&win), m).await;
        return;
    }

    // Make progress window modal.
    let pw = match pw.set_modal(&win) {
        Ok(v) => v,
        Err(e) => {
            let m = slint::format!(
                "Failed to make firmware progress window modal: {}.",
                e.display()
            );

            error(Some(&win), m).await;
            return;
        }
    };

    // Setup progress updater.
    let n = dump.items();
    let mut p = 0u32;
    let mut step = || {
        p += 1;
        pw.set_progress(p as f32 / n as f32);
        wae::yield_now()
    };

    wae::yield_now().await;

    // Extract.
    let e = loop {
        // Get next item.
        let mut item = match dump.next_item() {
            Ok(Some(v)) => v,
            Ok(None) => break None,
            Err(e) => break Some(FirmwareError::NextItem(e)),
        };

        // Update status.
        let name = item.to_string();

        pw.set_status(slint::format!("Extracting {name}..."));

        wae::yield_now().await;

        // Extract item.
        let r: Result<(), Box<dyn Error>> = match &mut item {
            ItemReader::Ps4Part(r) => extract_partition(&pw, &dmgr, r, &mut step)
                .await
                .map_err(|e| e.into()),
        };

        if let Err(e) = r {
            break Some(FirmwareError::ExtractItem(name, e));
        }

        step().await;
    };

    // Check status.
    if let Err(e) = pw.hide() {
        let m = slint::format!("Failed to close firmware progress window: {}.", e.display());
        error(Some(pw.deref()), m).await;
    }

    drop(pw);
    wae::yield_now().await;

    match e {
        Some(e) => {
            let m = slint::format!("Failed to install {}: {}.", path, e.display());
            error(Some(&win), m).await;
        }
        None => win.invoke_set_firmware_finished(),
    }
}

async fn extract_partition<F: Future<Output = ()>>(
    pw: &InstallFirmware,
    dmgr: &DataMgr,
    part: &mut PartReader<'_, File>,
    step: &mut impl FnMut() -> F,
) -> Result<(), PartitionError> {
    // Get FS type.
    let fs = match part.fs() {
        b"exfatfs" => FsType::ExFat,
        n => {
            let n = String::from_utf8_lossy(n);
            return Err(PartitionError::UnexpectedFs(n.into_owned()));
        }
    };

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

    // Create database file for file/directory metadata.
    let mp = dmgr.partitions().meta(dev);
    let meta = match File::create_new(&mp) {
        Ok(v) => v,
        Err(e) => return Err(PartitionError::CreateFile(mp, e)),
    };

    // Create metadata database.
    let meta = match Database::builder().create_file(meta) {
        Ok(v) => v,
        Err(e) => return Err(PartitionError::CreateMeta(mp, e)),
    };

    // Start metadata transaction.
    let meta = match meta.begin_write() {
        Ok(v) => v,
        Err(e) => return Err(PartitionError::MetaTransaction(mp, e)),
    };

    // Write FS type.
    let mut tab = match meta.open_table(FS_TYPE) {
        Ok(v) => v,
        Err(e) => return Err(PartitionError::MetaTable(mp, FS_TYPE.to_string(), e)),
    };

    if let Err(e) = tab.insert((), fs) {
        return Err(PartitionError::WriteFs(mp, e));
    }

    drop(tab);

    // Extract items.
    let root = dmgr.partitions().data(dev);
    let mut buf = vec![0u8; 0xFFFF];

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
                pw.set_status(slint::format!("Extracting {name}..."));

                wae::yield_now().await;

                // Create only if not exists.
                let mut file = match File::create_new(&path) {
                    Ok(v) => v,
                    Err(e) => return Err(PartitionError::CreateFile(path, e)),
                };

                // Copy data.
                loop {
                    let n = match data.read(&mut buf) {
                        Ok(v) => v,
                        Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => return Err(PartitionError::ExtractFile(name, path, e)),
                    };

                    if n == 0 {
                        break;
                    }

                    // Write file.
                    if let Err(e) = file.write_all(&buf[..n]) {
                        return Err(PartitionError::ExtractFile(name, path, e));
                    }

                    wae::yield_now().await;
                }
            }
            None => {
                // Create only if not exists.
                if let Err(e) = std::fs::create_dir(&path) {
                    return Err(PartitionError::CreateDirectory(path, e));
                }
            }
        }

        step().await;
    }

    // Commit metadata transaction.
    pw.set_status("Committing metadata database...".into());

    wae::yield_now().await;

    if let Err(e) = meta.commit() {
        return Err(PartitionError::MetaCommit(mp, e));
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
    #[error("unexpected filesystem {0}")]
    UnexpectedFs(String),

    #[error("unexpected device {0}")]
    UnexpectedDevice(String),

    #[error("couldn't create metadata database on {0}")]
    CreateMeta(PathBuf, #[source] DatabaseError),

    #[error("couldn't start metadata transaction on {0}")]
    MetaTransaction(PathBuf, #[source] redb::TransactionError),

    #[error("couldn't open table {1} on {0}")]
    MetaTable(PathBuf, String, #[source] redb::TableError),

    #[error("couldn't write filesystem type to {0}")]
    WriteFs(PathBuf, #[source] redb::StorageError),

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

    #[error("couldn't commit metadata transaction to {0}")]
    MetaCommit(PathBuf, #[source] redb::CommitError),
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

    #[error("couldn't center setup wizard")]
    CenterWindow(#[source] crate::ui::PlatformError),

    #[error("couldn't show setup wizard")]
    ShowWindow(#[source] PlatformError),
}
