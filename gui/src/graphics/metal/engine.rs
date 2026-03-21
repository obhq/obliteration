// SPDX-License-Identifier: MIT OR Apache-2.0
use super::GraphicsError;
use crate::graphics::Graphics;
use metal::{Device, MetalLayer};

/// Implementation of [`Graphics`] using Metal.
///
/// Fields in this struct need to be dropped in a correct order.
pub struct Metal {
    device: Device,
}

impl Metal {
    pub fn new() -> Result<Self, GraphicsError> {
        // Get Metal device.
        let device = match Device::system_default() {
            Some(v) => v,
            None => return Err(GraphicsError::GetDeviceFailed),
        };

        Ok(Self { device })
    }

    /// # Safety
    /// The returned [`MetalLayer`] must be dropped before this [`Metal`].
    pub unsafe fn create_layer(&self) -> MetalLayer {
        let layer = MetalLayer::new();

        layer.set_device(&self.device);
        layer
    }
}

impl Graphics for Metal {}
