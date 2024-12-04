use std::ffi::OsString;
use std::io::ErrorKind;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use thiserror::Error;
use xdg::BaseDirectories;

pub fn read_data_root() -> Result<Option<PathBuf>, DataRootError> {
    // Read config file.
    let file = get_config_path()?;
    let mut path = match std::fs::read(&file) {
        Ok(v) => v,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(DataRootError::ReadFile(file, e)),
    };

    // Trim leading whitespaces.
    let mut len = 0;

    for b in &path {
        if !b.is_ascii_whitespace() {
            break;
        }

        len += 1;
    }

    path.drain(..len);

    // Trim trailing whitespaces.
    while path.last().is_some_and(|b| b.is_ascii_whitespace()) {
        path.pop();
    }

    Ok(Some(OsString::from_vec(path).into()))
}

pub fn write_data_root(path: impl AsRef<Path>) -> Result<(), DataRootError> {
    let file = get_config_path()?;
    let path = path.as_ref().as_os_str();

    if let Err(e) = std::fs::write(&file, path.as_bytes()) {
        return Err(DataRootError::WriteFile(file, e));
    }

    Ok(())
}

fn get_config_path() -> Result<PathBuf, DataRootError> {
    BaseDirectories::new()
        .map(|xdg| {
            let mut p = xdg.get_config_home();
            p.push("obliteration.conf");
            p
        })
        .map_err(DataRootError::XdgBaseDirectory)
}

/// Represents an error when read or write data root fails.
#[derive(Debug, Error)]
pub enum DataRootError {
    #[error("couldn't load XDG Base Directory")]
    XdgBaseDirectory(#[source] xdg::BaseDirectoriesError),

    #[error("couldn't read {0}")]
    ReadFile(PathBuf, #[source] std::io::Error),

    #[error("couldn't write {0}")]
    WriteFile(PathBuf, #[source] std::io::Error),
}
