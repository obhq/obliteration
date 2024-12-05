use std::io::ErrorKind;
use std::path::PathBuf;
use thiserror::Error;
use xdg::BaseDirectories;

pub fn read_data_root() -> Result<Option<String>, DataRootError> {
    // Read config file.
    let file = get_config_path()?;
    let mut path = match std::fs::read_to_string(&file) {
        Ok(v) => v,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(DataRootError::ReadFile(file, e)),
    };

    // Trim trailing whitespaces before leading whitespaces so the latter don't need to move
    // trailing whitespaces that going to remove anyway.
    let count = path.chars().rev().take_while(|c| c.is_whitespace()).count();

    for _ in 0..count {
        path.pop();
    }

    // Trim leading whitespaces.
    let count = path.chars().take_while(|c| c.is_whitespace()).count();

    for _ in 0..count {
        path.remove(0);
    }

    Ok(Some(path))
}

pub fn write_data_root(path: impl AsRef<str>) -> Result<(), DataRootError> {
    let file = get_config_path()?;

    if let Err(e) = std::fs::write(&file, path.as_ref()) {
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
