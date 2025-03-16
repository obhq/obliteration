use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use thiserror::Error;

/// Contains application settings.
#[derive(Default, Serialize, Deserialize)]
pub struct Settings {
    graphics_debug_layer: Cell<bool>,
}

impl Settings {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SettingsError> {
        // Open file.
        let path = path.as_ref();
        let file = match File::open(&path) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(SettingsError::OpenFile(e)),
        };

        // Read file.
        let data = match ciborium::from_reader(file) {
            Ok(v) => v,
            Err(e) => return Err(SettingsError::ReadFile(e)),
        };

        Ok(data)
    }

    pub fn graphics_debug_layer(&self) -> bool {
        self.graphics_debug_layer.get()
    }

    pub fn set_graphics_debug_layer(&self, v: bool) {
        self.graphics_debug_layer.set(v);
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SettingsError> {
        let path = path.as_ref();
        let file = match File::create(&path) {
            Ok(v) => v,
            Err(e) => return Err(SettingsError::CreateFile(e)),
        };

        ciborium::into_writer(self, file).map_err(SettingsError::WriteFile)
    }
}

/// Represents an error when [`Settings`] fails to load or save.
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("couldn't open the file")]
    OpenFile(#[source] std::io::Error),

    #[error("couldn't load the file")]
    ReadFile(#[source] ciborium::de::Error<std::io::Error>),

    #[error("couldn't create the file")]
    CreateFile(#[source] std::io::Error),

    #[error("couldn't write the file")]
    WriteFile(#[source] ciborium::ser::Error<std::io::Error>),
}
