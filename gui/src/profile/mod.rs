pub use self::ui::*;

use obconf::Config;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::num::NonZero;
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

mod ui;

/// Contains settings to launch the kernel.
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Profile {
    id: Uuid,
    name: String,
    display_resolution: DisplayResolution,
    kernel_config: Config,
    created: SystemTime,
}

impl Profile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadError> {
        let path = path.as_ref().join("profile.bin");

        let file = File::open(&path).map_err(LoadError::Open)?;

        let profile = ciborium::from_reader(file).map_err(LoadError::Load)?;

        Ok(profile)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SaveError> {
        let path = path.as_ref();

        std::fs::create_dir_all(path).map_err(SaveError::CreateDir)?;

        let path = path.join("profile.bin");

        let file = File::create(path).map_err(SaveError::CreateFile)?;

        ciborium::into_writer(self, file).map_err(SaveError::WriteFile)?;

        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn kernel_config(&self) -> &Config {
        &self.kernel_config
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from("Default"),
            display_resolution: DisplayResolution::Hd,
            kernel_config: Config {
                max_cpu: NonZero::new(8).unwrap(),
            },
            created: SystemTime::now(),
        }
    }
}

/// Display resolution to report to the kernel.
#[derive(Clone, Copy, Deserialize, Serialize)]
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
    #[error("couldn't open the profile file")]
    Open(#[source] std::io::Error),

    #[error("couldn't load the profile file")]
    Load(#[source] ciborium::de::Error<std::io::Error>),
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("couldn't create the directory")]
    CreateDir(#[source] std::io::Error),

    #[error("couldn't create the profile file")]
    CreateFile(#[source] std::io::Error),

    #[error("couldn't write the profile file")]
    WriteFile(#[source] ciborium::ser::Error<std::io::Error>),
}
