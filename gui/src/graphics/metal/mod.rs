// SPDX-License-Identifier: MIT OR Apache-2.0
use self::screen::MetalScreen;
use super::Graphics;
use metal::Device;
use std::ops::Deref;
use thiserror::Error;

mod buffer;
mod screen;

pub struct Metal {
    devices: Vec<metal::Device>,
}

impl Graphics for Metal {
    type Err = MetalError;
    type PhysicalDevice = metal::Device;
    type Screen = MetalScreen;

    fn new() -> Result<Self, Self::Err> {
        Ok(Self {
            devices: Device::all(),
        })
    }

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn create_screen(&mut self) -> Result<Self::Screen, Self::Err> {
        todo!()
    }
}

impl super::PhysicalDevice for metal::Device {
    fn name(&self) -> &str {
        self.deref().name()
    }
}

/// Implementation of [`Graphics::Err`] for Metal.
#[derive(Debug, Error)]
pub enum MetalError {}
