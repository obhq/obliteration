use crate::exe::Executable;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub struct Process {}

impl Process {
    pub fn load(exe: Executable) -> Result<Self, LoadError> {
        Ok(Self {})
    }
}

#[derive(Debug)]
pub enum LoadError {}

impl Error for LoadError {}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Ok(())
    }
}
