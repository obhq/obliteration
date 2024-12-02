pub use self::part::*;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use thiserror::Error;

mod part;

/// Manages all files and directories that stored in the data root.
pub struct DataMgr {
    root: PathBuf,
    part: Part,
}

impl DataMgr {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, DataError> {
        // Build path for top-level items.
        let root: PathBuf = root.into();
        let part = root.join("part");

        // Create top-level directories.
        Self::create_dir(&part)?;

        Ok(Self {
            root,
            part: Part::new(part),
        })
    }

    pub fn part(&self) -> &Part {
        &self.part
    }

    fn create_dir(path: &Path) -> Result<(), DataError> {
        if let Err(e) = std::fs::create_dir(path) {
            if e.kind() != ErrorKind::AlreadyExists {
                return Err(DataError::CreateDirectory(path.to_owned(), e));
            }
        }

        Ok(())
    }
}

/// Represents an error when [`DataMgr`] fails to construct.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("couldn't create {0}")]
    CreateDirectory(PathBuf, #[source] std::io::Error),
}
