// SPDX-License-Identifier: MIT OR Apache-2.0
use self::engine::Metal;
use super::EngineBuilder;
use crate::profile::Profile;
use metal::Device;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thiserror::Error;
use winit::window::WindowAttributes;

mod engine;
mod window;

pub fn builder() -> Result<impl EngineBuilder, GraphicsError> {
    Ok(MetalBuilder {
        devices: Device::all(),
    })
}

/// Implementation of [`EngineBuilder`] for Metal.
struct MetalBuilder {
    devices: Vec<metal::Device>,
}

impl EngineBuilder for MetalBuilder {
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
    fn name(&self) -> &str {
        self.deref().name()
    }
}

/// Represents an error when operation on Metal fails.
#[derive(Debug, Error)]
pub enum GraphicsError {}
