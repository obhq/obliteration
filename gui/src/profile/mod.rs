use config::Config;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

/// Contains settings to launch the kernel.
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Profile {
    id: Uuid,
    name: String,
    display_device: ByteBuf,
    display_resolution: DisplayResolution,
    debug_addr: SocketAddr,
    kernel_config: Config,
    created: SystemTime,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            display_device: ByteBuf::new(),
            display_resolution: DisplayResolution::Hd,
            debug_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1234)),
            kernel_config: Config {
                max_cpu: NonZero::new(8).unwrap(),
                ..Default::default()
            },
            created: SystemTime::now(),
        }
    }

    pub fn load(root: impl AsRef<Path>) -> Result<Self, LoadError> {
        // Open profile.
        let root = root.as_ref();
        let path = root.join("profile.bin");
        let file = match File::open(&path) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::OpenFile(path, e)),
        };

        // Read profile.
        let profile = match ciborium::from_reader(file) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::ReadProfile(path, e)),
        };

        Ok(profile)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn display_device(&self) -> &[u8] {
        &self.display_device
    }

    pub fn set_display_device(&mut self, v: impl Into<Vec<u8>>) {
        self.display_device = v.into().into();
    }

    pub fn display_resolution(&self) -> DisplayResolution {
        self.display_resolution
    }

    pub fn set_display_resolution(&mut self, v: DisplayResolution) {
        self.display_resolution = v;
    }

    pub fn debug_addr(&self) -> &SocketAddr {
        &self.debug_addr
    }

    pub fn set_debug_addr(&mut self, v: SocketAddr) {
        self.debug_addr = v;
    }

    pub fn kernel_config(&self) -> &Config {
        &self.kernel_config
    }

    pub fn save(&self, root: impl AsRef<Path>) -> Result<(), SaveError> {
        // Write profile.
        let root = root.as_ref();
        let path = root.join("profile.bin");
        let file = match File::create(&path) {
            Ok(v) => v,
            Err(e) => return Err(SaveError::CreateFile(path, e)),
        };

        if let Err(e) = ciborium::into_writer(self, file) {
            return Err(SaveError::WriteProfile(path, e));
        }

        Ok(())
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::new("Default")
    }
}

/// Display resolution to report to the kernel.
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum DisplayResolution {
    /// 1280 × 720.
    Hd,
    /// 1920 × 1080.
    FullHd,
    /// 3840 × 2160.
    UltraHd,
}

impl Display for DisplayResolution {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Self::Hd => "1280 × 720",
            Self::FullHd => "1920 × 1080",
            Self::UltraHd => "3840 × 2160",
        };

        f.write_str(v)
    }
}

/// Represents an error when [`Profile::load()`] fails.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("couldn't open {0}")]
    OpenFile(PathBuf, #[source] std::io::Error),

    #[error("couldn't read {0}")]
    ReadProfile(PathBuf, #[source] ciborium::de::Error<std::io::Error>),
}

/// Represents an error when [`Profile::save()`] fails.
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("couldn't create {0}")]
    CreateFile(PathBuf, #[source] std::io::Error),

    #[error("couldn't write {0}")]
    WriteProfile(PathBuf, #[source] ciborium::ser::Error<std::io::Error>),
}
