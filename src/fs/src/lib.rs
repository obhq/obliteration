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

        file.create_new(true);
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
    #[derive(Clone)]
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

impl<'de> serde::Deserialize<'de> for FileMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mode = u16::deserialize(deserializer)?;
        Self::from_bits(mode)
            .ok_or_else(|| serde::de::Error::custom("Invalid value for Deserialization."))
    }
}

impl serde::Serialize for FileMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u16(self.bits())
    }
}

impl From<u16> for FileMode {
    fn from(item: u16) -> Self {
        Self::from_bits_truncate(item)
    }
}

/// Contains some information for SPRX file.
#[derive(Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
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
