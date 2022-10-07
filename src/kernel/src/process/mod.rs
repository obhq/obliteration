use crate::fs::file::File;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub struct Process {
    bin: File,
}

impl Process {
    pub(super) fn load(bin: File) -> Result<Self, LoadError> {
        Ok(Self { bin })
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
