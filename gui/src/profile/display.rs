use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

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
