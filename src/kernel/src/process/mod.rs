use crate::exe::{Class, Executable};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub struct Process {}

impl Process {
    pub fn load(exe: Executable) -> Result<Self, LoadError> {
        // All PS4 exe should be 64-bits.
        if exe.class() != Class::SIXTY_FOUR_BITS {
            return Err(LoadError::UnsupportedClass);
        }

        Ok(Self {})
    }
}

#[derive(Debug)]
pub enum LoadError {
    UnsupportedClass,
}

impl Error for LoadError {}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::UnsupportedClass => f.write_str("unsupported executable class"),
        }
    }
}
