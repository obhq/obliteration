pub use self::cpu::*;
pub use self::display::*;

use config::Config;
use minicbor_serde::error::DecodeError;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::fs::File;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

mod cpu;
mod display;

/// Contains settings to launch the kernel.
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Profile {
    id: Uuid,
    pub name: String,
    pub display_device: ByteBuf,
    pub display_resolution: DisplayResolution,
    pub cpu_model: CpuModel,
    pub debug_addr: SocketAddr,
    pub kernel_config: Box<Config>,
    created: SystemTime,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            display_device: ByteBuf::new(),
            display_resolution: DisplayResolution::Hd,
            cpu_model: CpuModel::Pro,
            debug_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1234)),
            kernel_config: Box::new(Config {
                max_cpu: NonZero::new(8).unwrap(),
                ..Default::default()
            }),
            created: SystemTime::now(),
        }
    }

    pub fn load(root: impl AsRef<Path>) -> Result<Self, LoadError> {
        // Open profile.
        let root = root.as_ref();
        let path = root.join("profile.bin");
        let data = match std::fs::read(&path) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::ReadFile(path, e)),
        };

        // Read profile.
        let profile = match minicbor_serde::from_slice(&data) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::LoadProfile(path, e)),
        };

        Ok(profile)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn save(&self, root: impl AsRef<Path>) -> Result<(), SaveError> {
        // Write profile.
        let root = root.as_ref();
        let path = root.join("profile.bin");
        let mut file = match File::create(&path) {
            Ok(v) => v,
            Err(e) => return Err(SaveError::CreateFile(path, e)),
        };

        file.write_all(&minicbor_serde::to_vec(self).unwrap())
            .map_err(|e| SaveError::WriteProfile(path, e))
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::new("Default")
    }
}

/// Represents an error when [`Profile::load()`] fails.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("couldn't read {0}")]
    ReadFile(PathBuf, #[source] std::io::Error),

    #[error("couldn't load {0}")]
    LoadProfile(PathBuf, #[source] DecodeError),
}

/// Represents an error when [`Profile::save()`] fails.
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("couldn't create {0}")]
    CreateFile(PathBuf, #[source] std::io::Error),

    #[error("couldn't write {0}")]
    WriteProfile(PathBuf, #[source] std::io::Error),
}
