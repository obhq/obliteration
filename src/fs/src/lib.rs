use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use thiserror::Error;

/// Contains medata for a file in the PS4 system.
#[derive(Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub mode: FileMode,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub birthtime: u64,
    pub mtimensec: u32,
    pub atimensec: u32,
    pub ctimensec: u32,
    pub birthnsec: u32,
    pub uid: u32,
    pub gid: u32,
}

impl Metadata {
    pub fn create_for<F: Into<PathBuf>>(&self, file: F) -> Result<(), CreateForError> {
        // Create path for metadata.
        let mut path = file.into();
        let mut name = match path.file_name() {
            Some(v) => v.to_os_string(),
            None => return Err(CreateForError::InvalidFilePath),
        };

        name.push(".obm"); // Let's hope no any games using "obm" as a file extension.
        path.set_file_name(name);

        // Create metadata file.
        let mut file = std::fs::OpenOptions::new();

        file.create(true);
        file.truncate(true);
        file.write(true);

        let file = match file.open(&path) {
            Ok(v) => v,
            Err(e) => return Err(CreateForError::CreateMetadataFailed(path, e)),
        };

        // Write metadata.
        if let Err(e) = serde_yaml::to_writer(file, self) {
            return Err(CreateForError::WriteMetadataFailed(path, e.into()));
        }

        Ok(())
    }
}

bitflags! {
    /// Unix mode of a game file.
    ///
    /// The value of this is exactly the same as the value in the PFS.
    #[derive(Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    #[repr(transparent)]
    pub struct FileMode: u16 {
        const S_IXOTH = 0b0000000000000001;
        const S_IWOTH = 0b0000000000000010;
        const S_IROTH = 0b0000000000000100;
        const S_IXGRP = 0b0000000000001000;
        const S_IWGRP = 0b0000000000010000;
        const S_IRGRP = 0b0000000000100000;
        const S_IXUSR = 0b0000000001000000;
        const S_IWUSR = 0b0000000010000000;
        const S_IRUSR = 0b0000000100000000;
    }
}

impl From<u16> for FileMode {
    fn from(item: u16) -> Self {
        Self::from_bits_retain(item)
    }
}

/// Errors for [`create_for()`][Metadata::create_for()].
#[derive(Debug, Error)]
pub enum CreateForError {
    #[error("file path is not valid")]
    InvalidFilePath,

    #[error("cannot create {0}")]
    CreateMetadataFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot write {0}")]
    WriteMetadataFailed(PathBuf, #[source] Box<dyn Error>),
}
