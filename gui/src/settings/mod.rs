use minicbor_serde::error::DecodeError;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::fs::File;
use std::io::{ErrorKind, Write};
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
        let data = match std::fs::read(path) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(SettingsError::ReadFile(e)),
        };

        // Read file.
        let data = match minicbor_serde::from_slice(&data) {
            Ok(v) => v,
            Err(e) => return Err(SettingsError::LoadFile(e)),
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
        let mut file = match File::create(&path) {
            Ok(v) => v,
            Err(e) => return Err(SettingsError::CreateFile(e)),
        };

        file.write_all(&minicbor_serde::to_vec(self).unwrap())
            .map_err(SettingsError::WriteFile)
    }
}

/// Represents an error when [`Settings`] fails to load or save.
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("couldn't read the file")]
    ReadFile(#[source] std::io::Error),

    #[error("couldn't load the file")]
    LoadFile(#[source] DecodeError),

    #[error("couldn't create the file")]
    CreateFile(#[source] std::io::Error),

    #[error("couldn't write the file")]
    WriteFile(#[source] std::io::Error),
}
