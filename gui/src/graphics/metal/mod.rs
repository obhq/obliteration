// SPDX-License-Identifier: MIT OR Apache-2.0
use self::screen::MetalScreen;
use super::Graphics;
use crate::profile::Profile;
use metal::Device;
use std::ops::Deref;
use thiserror::Error;

mod buffer;
mod screen;

pub fn new() -> Result<impl Graphics, GraphicsError> {
    Ok(Metal {
        devices: Device::all(),
    })
}

/// Implementation of [`Graphics`] using Metal.
struct Metal {
    devices: Vec<metal::Device>,
}

impl Graphics for Metal {
    type PhysicalDevice = metal::Device;
    type Screen = MetalScreen;

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn create_screen(&mut self, profile: &Profile) -> Result<Self::Screen, GraphicsError> {
        todo!()
    }
}

impl super::PhysicalDevice for metal::Device {
    fn name(&self) -> &str {
        self.deref().name()
    }
}

/// Represents an error when operation on Metal fails.
#[derive(Debug, Error)]
pub enum GraphicsError {}
