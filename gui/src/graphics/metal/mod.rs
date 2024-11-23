// SPDX-License-Identifier: MIT OR Apache-2.0
use metal::Device;
use std::ops::Deref;
use thiserror::Error;

pub struct Metal {
    devices: Vec<metal::Device>,
}

impl super::GraphicsApi for Metal {
    type PhysicalDevice = metal::Device;

    type CreateError = MetalCreateError;

    fn new() -> Result<Self, Self::CreateError> {
        Ok(Self {
            devices: Device::all(),
        })
    }

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }
}

impl super::PhysicalDevice for metal::Device {
    fn name(&self) -> &str {
        self.deref().name()
    }
}

/// Represents an error when [`Metal::new()`] fails.
#[derive(Debug, Error)]
pub enum MetalCreateError {}
