// SPDX-License-Identifier: MIT OR Apache-2.0
use self::buffer::MetalBuffer;
use super::{Screen, ScreenBuffer};
use crate::vmm::VmmScreen;
use metal::{CAMetalLayer, Device, MetalLayer};
use objc::runtime::{Object, NO, YES};
use objc::{msg_send, sel, sel_impl};
use std::ptr::null_mut;
use std::sync::Arc;
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
        self.name()
    }
}

/// Represents an error when [`Metal::new()`] fails.
#[derive(Debug, Error)]
pub enum MetalCreateError {}
