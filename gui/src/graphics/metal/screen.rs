// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::graphics::Screen;
use metal::{Device, MetalLayer};
use thiserror::Error;

/// Implementation of [`Screen`] using Metal.
///
/// Fields in this struct need to be dropped in a correct order.
pub struct MetalScreen {
    device: Device,
}

impl MetalScreen {
    pub fn new() -> Result<Self, MetalError> {
        // Get Metal device.
        let device = match Device::system_default() {
            Some(v) => v,
            None => return Err(MetalError::GetDeviceFailed),
        };

        Ok(Self { device })
    }

    /// # Safety
    /// The returned [`MetalLayer`] must be dropped before this [`MetalScreen`].
    pub unsafe fn create_layer(&self) -> MetalLayer {
        let layer = MetalLayer::new();

        layer.set_device(&self.device);
        layer
    }
}

impl Screen for MetalScreen {}

/// Represents an error when [`MetalScreen::new()`] fails.
#[derive(Debug, Error)]
pub enum MetalError {
    #[error("couldn't get default MTLDevice")]
    GetDeviceFailed,
}
