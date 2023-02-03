use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub mod path;

/// A virtual filesystem for emulating a PS4 filesystem.
pub struct Fs {
    mounts: RwLock<HashMap<String, MountPoint>>,
}

impl Fs {
    pub(super) fn new() -> Self {
        Self {
            mounts: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &str) -> Result<Item, GetError> {
        // Check if path absolute.
        if !path.starts_with('/') {
            return Err(GetError::InvalidPath);
        }

        // Get root mount point.
        let mut current = String::with_capacity(path.len());

        current.push('/');

        let mounts = self.mounts.read().unwrap();
        let mount = match mounts.get(&current) {
            Some(v) => v,
            None => return Err(GetError::NoRootFs),
        };

        // Open a root directory.
        let mut directory = Directory {
            path: mount.path.clone(),
        };

        // Walk on virtual path components.
        for component in path::decompose(&path[1..]) {
            current.push_str(component);

            // Check if a virtual path is a mount point.
            if let Some(v) = mounts.get(&current) {
                directory = Directory {
                    path: v.path.clone(),
                };
            } else {
                // Build a real path.
                let mut path = directory.path;

                path.push(component);

                // Check if path is a file.
                if path.is_file() {
                    return Ok(Item::File(File { path }));
                }

                directory = Directory { path };
            }

            current.push('/');
        }

        // If we reached here that mean the the last component is a directory.
        Ok(Item::Directory(directory))
    }

    pub fn mount<T>(&self, target: T, data: MountPoint) -> Result<(), MountError>
    where
        T: Into<String>,
    {
        use std::collections::hash_map::Entry;

        let mut mounts = self.mounts.write().unwrap();

        match mounts.entry(target.into()) {
            Entry::Occupied(_) => return Err(MountError::AlreadyMounted),
            Entry::Vacant(e) => e.insert(data),
        };

        Ok(())
    }
}

/// An item in the virtual filesystem.
pub enum Item {
    Directory(Directory),
    File(File),
}

/// A virtual directory.
pub struct Directory {
    path: PathBuf,
}

/// A virtual file.
pub struct File {
    path: PathBuf,
}

impl File {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A mount point in the virtual filesystem.
pub struct MountPoint {
    path: PathBuf,
}

impl MountPoint {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Debug)]
pub enum GetError {
    InvalidPath,
    NoRootFs,
}

impl Error for GetError {}

impl Display for GetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidPath => f.write_str("invalid path"),
            Self::NoRootFs => f.write_str("no rootfs mounted"),
        }
    }
}

#[derive(Debug)]
pub enum MountError {
    AlreadyMounted,
}

impl Error for MountError {}

impl Display for MountError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::AlreadyMounted => f.write_str("target is already mounted"),
        }
    }
}
