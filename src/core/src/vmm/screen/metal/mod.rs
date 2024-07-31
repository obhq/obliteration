use super::Screen;
use crate::vmm::VmmError;
use thiserror::Error;

/// Implementation of [`Screen`] using Metal.
pub struct Metal {}

impl Metal {
    pub fn new(surface: usize) -> Result<Self, VmmError> {
        Ok(Self {})
    }
}

impl Screen for Metal {
    type UpdateErr = UpdateError;

    fn update(&self) -> Result<(), Self::UpdateErr> {
        todo!()
    }
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
