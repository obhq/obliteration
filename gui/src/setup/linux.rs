use std::io::ErrorKind;
use std::path::PathBuf;
use thiserror::Error;
use xdg::BaseDirectories;

pub fn read_data_root() -> Result<Option<PathBuf>, DataRootError> {
    let file = get_config_path()?;

    match std::fs::read_to_string(&file) {
        Ok(v) => Ok(Some(v.trim().into())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(DataRootError::ReadFile(file, e)),
    }
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
}
