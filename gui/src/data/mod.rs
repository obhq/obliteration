pub use self::part::*;
pub use self::prof::*;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use thiserror::Error;

mod part;
mod prof;

/// Manages all files and directories that stored in the data root.
///
/// This does not manage file content. Its job is to organize filesystem hierarchy.
pub struct DataMgr {
    part: Part,
    prof: Prof,
    logs: PathBuf,
}

impl DataMgr {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, DataError> {
        // Build path for top-level items.
        let root: PathBuf = root.into();
        let part = root.join("part");
        let prof = root.join("prof");
        let logs = root.join("kernel.txt");

        // Create top-level directories.
        Self::create_dir(&part)?;
        Self::create_dir(&prof)?;

        Ok(Self {
            part: Part::new(part),
            prof: Prof::new(prof),
            logs,
        })
    }

    pub fn partitions(&self) -> &Part {
        &self.part
    }

    pub fn profiles(&self) -> &Prof {
        &self.prof
    }

    pub fn logs(&self) -> &Path {
        &self.logs
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

/// Represents an error when operation on data root fails.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("couldn't create {0}")]
    CreateDirectory(PathBuf, #[source] std::io::Error),

    #[error("couldn't list item in {0}")]
    ReadDirectory(PathBuf, #[source] std::io::Error),
}
