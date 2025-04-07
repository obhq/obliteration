// SPDX-License-Identifier: MIT OR Apache-2.0
use self::engine::Metal;
use super::GraphicsBuilder;
use crate::profile::Profile;
use crate::settings::Settings;
use metal::Device;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thiserror::Error;
use winit::window::WindowAttributes;

mod engine;
mod window;

pub fn builder(settings: &Settings) -> Result<impl GraphicsBuilder, GraphicsError> {
    Ok(MetalBuilder {
        devices: Device::all(),
    })
}

/// Implementation of [`GraphicsBuilder`] for Metal.
struct MetalBuilder {
    devices: Vec<metal::Device>,
}

impl GraphicsBuilder for MetalBuilder {
    type PhysicalDevice = metal::Device;
    type Engine = Metal;

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn build(
        self,
        profile: &Profile,
        screen: WindowAttributes,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Arc<Self::Engine>, GraphicsError> {
        todo!()
    }
}

impl super::PhysicalDevice for metal::Device {
    fn id(&self) -> &[u8] {
        todo!()
    }

    fn name(&self) -> &str {
        self.deref().name()
    }
}

/// Represents an error when operation on Metal fails.
#[derive(Debug, Error)]
pub enum GraphicsError {}
