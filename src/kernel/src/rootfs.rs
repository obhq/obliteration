use crate::fs::driver::{Driver, Entry, OpenError};
use std::marker::PhantomData;

/// Represents a virtual root file system. The directory structure the kernel will see will be the
/// same as PS4 while the actual structure in the host will be different.
pub(super) struct RootFs {}

impl RootFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl Driver for RootFs {
    fn open_root(&self, _: &str) -> Result<Box<dyn crate::fs::driver::Directory + '_>, OpenError> {
        Ok(Box::new(Directory {
            phantom: PhantomData,
        }))
    }
}

struct Directory<'driver> {
    phantom: PhantomData<&'driver RootFs>,
}

impl<'driver> crate::fs::driver::Directory<'driver> for Directory<'driver> {
    fn open(&self, _: &str) -> Result<Entry<'driver>, OpenError> {
        Err(OpenError::NotFound)
    }

    fn to_token(&self) -> Box<dyn crate::fs::driver::DirectoryToken> {
        Box::new(DirectoryToken {})
    }
}

struct DirectoryToken;

impl crate::fs::driver::DirectoryToken for DirectoryToken {}
