use super::PlatformExt;
use slint::ComponentHandle;
use thiserror::Error;

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        todo!()
    }
}

/// Windows-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
