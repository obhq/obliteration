use super::Screen;
use crate::vmm::VmmError;
use thiserror::Error;

/// Implementation of [`Screen`] using Vulkan.
pub struct Vulkan {}

impl Vulkan {
    pub fn new(surface: usize) -> Result<Self, VmmError> {
        Ok(Self {})
    }
}

impl Screen for Vulkan {
    type UpdateErr = UpdateError;

    fn update(&self) -> Result<(), Self::UpdateErr> {
        todo!()
    }
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
