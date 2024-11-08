use obconf::Config;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::num::NonZero;
use std::time::SystemTime;
use uuid::Uuid;

#[cfg(feature = "qt_ffi")]
mod ffi;

/// Contains settings to launch the kernel.
#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Profile {
    id: Uuid,
    name: CString,
    display_resolution: DisplayResolution,
    kernel_config: Config,
    created: SystemTime,
}

impl Profile {
    pub fn name(&self) -> &CStr {
        &self.name
    }

    pub fn kernel_config(&self) -> &Config {
        &self.kernel_config
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: CString::new("Default").unwrap(),
            display_resolution: DisplayResolution::Hd,
            kernel_config: Config {
                max_cpu: NonZero::new(8).unwrap(),
            },
            created: SystemTime::now(),
        }
    }
}

/// Display resolution to report to the kernel.
#[repr(C)]
#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum DisplayResolution {
    /// 1280 × 720.
    Hd,
    /// 1920 × 1080.
    FullHd,
    /// 3840 × 2160.
    UltraHd,
}
