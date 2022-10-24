use std::error::Error;
use std::fmt::{Display, Formatter};

pub struct ExFat<'image> {
    image: &'image [u8],
}

impl<'image> ExFat<'image> {
    pub fn open(image: &'image [u8]) -> Result<Self, OpenError> {
        Ok(Self { image })
    }
}

#[derive(Debug)]
pub enum OpenError {}

impl Error for OpenError {}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
