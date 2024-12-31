use super::PlatformExt;
use crate::rt::RuntimeWindow;
use slint::ComponentHandle;
use thiserror::Error;

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        todo!()
    }

    fn set_modal<P>(&self, parent: &P) -> Result<(), PlatformError>
    where
        P: RuntimeWindow + ?Sized,
    {
        todo!()
    }
}

/// Windows-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
