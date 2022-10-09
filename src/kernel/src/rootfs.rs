use crate::fs::driver::{self, Driver, Entry, OpenError};
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
    fn open_root(&self, path: &str) -> Result<Box<dyn driver::Directory + '_>, OpenError> {
        Ok(Box::new(Directory {
            path: path.into(),
            phantom: PhantomData,
        }))
    }
}

struct Directory<'driver> {
    path: String,
    phantom: PhantomData<&'driver RootFs>,
}

impl<'driver> driver::Directory<'driver> for Directory<'driver> {
    fn open(&self, name: &str) -> Result<Entry<'driver>, OpenError> {
        // Build full path.
        let path = if self.path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", self.path, name)
        };

        // Map entry.
        let entry = match path.as_str() {
            "/mnt" | "/mnt/app0" => Entry::Directory(Box::new(Directory {
                path,
                phantom: PhantomData,
            })),
            _ => return Err(OpenError::NotFound),
        };

        Ok(entry)
    }
}
