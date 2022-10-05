use super::RootFs;
use crate::fs::{Directory, Fs, Item, MountError, OpenError};
use std::sync::Arc;

/// Represents a root ("/") directory.
pub(super) struct Root<'fs> {
    fs: &'fs RootFs<'fs>,
}

impl<'fs> Root<'fs> {
    pub fn new(fs: &'fs RootFs<'fs>) -> Self {
        Self { fs }
    }
}

impl<'fs> Directory<'fs> for Root<'fs> {
    fn open(&self, name: &str) -> Result<Item<'fs>, OpenError> {
        if let Some(v) = self.fs.mounts.get("/", name) {
            return Ok(Item::Fs(v));
        }

        match name {
            "mnt" => Ok(Item::Directory(Box::new(Mnt {
                fs: self.fs,
                path: "/mnt".into(),
            }))),
            _ => Err(OpenError::NotFound),
        }
    }

    fn mount(&self, _: Arc<dyn Fs<'fs> + 'fs>) -> Result<(), MountError<'fs>> {
        Err(MountError::RootDirectory)
    }
}

/// Represents /mnt directory.
struct Mnt<'fs> {
    fs: &'fs RootFs<'fs>,
    path: String,
}

impl<'fs> Directory<'fs> for Mnt<'fs> {
    fn open(&self, name: &str) -> Result<Item<'fs>, OpenError> {
        if let Some(v) = self.fs.mounts.get(&self.path, name) {
            return Ok(Item::Fs(v));
        }

        match name {
            // Not sure if it is a symlink but let's use directory for now until it does not work.
            "app0" => Ok(Item::Directory(Box::new(App0 {
                fs: self.fs,
                path: format!("{}/{}", self.path, name),
            }))),
            _ => Err(OpenError::NotFound),
        }
    }

    fn mount(&self, fs: Arc<dyn Fs<'fs> + 'fs>) -> Result<(), MountError<'fs>> {
        self.fs.mounts.insert(&self.path, fs)
    }
}

/// Represents /mnt/app0, which is a directory (or symlink?) contains a data from PFS image for the
/// current running app.
struct App0<'fs> {
    fs: &'fs RootFs<'fs>,
    path: String,
}

impl<'fs> Directory<'fs> for App0<'fs> {
    fn open(&self, _: &str) -> Result<Item<'fs>, OpenError> {
        Err(OpenError::NotFound)
    }

    fn mount(&self, fs: Arc<dyn Fs<'fs> + 'fs>) -> Result<(), MountError<'fs>> {
        self.fs.mounts.insert(&self.path, fs)
    }
}
