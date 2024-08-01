use self::buffer::VulkanBuffer;
use super::{Screen, ScreenBuffer, VmmError};
use std::sync::Arc;
use thiserror::Error;

mod buffer;

/// Implementation of [`Screen`] using Vulkan.
pub struct Vulkan {
    buffer: Arc<VulkanBuffer>,
}

impl Vulkan {
    pub fn new(surface: usize) -> Result<Self, VmmError> {
        Ok(Self {
            buffer: Arc::new(VulkanBuffer::new()),
        })
    }
}

impl Screen for Vulkan {
    type Buffer = VulkanBuffer;
    type UpdateErr = UpdateError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn update(&mut self) -> Result<(), Self::UpdateErr> {
        todo!()
    }
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
