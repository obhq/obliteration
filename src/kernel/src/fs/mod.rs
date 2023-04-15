use self::path::{Vpath, VpathBuf};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use thiserror::Error;

pub mod path;

/// A virtual filesystem for emulating a PS4 filesystem.
pub struct Fs {
    mounts: RwLock<HashMap<VpathBuf, PathBuf>>,
}

impl Fs {
    pub(super) fn new() -> Self {
        Self {
            mounts: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &Vpath) -> Option<Item> {
        // Get root mount point.
        let mut current = VpathBuf::new();

        let mounts = self.mounts.read().unwrap();
        let root = match mounts.get(&current) {
            Some(v) => v,
            None => panic!("No rootfs is mounted."),
        };

        // Open a root directory.
        let mut directory = Directory {
            path: root.clone(),
            virtual_path: VpathBuf::new(),
        };

        // Walk on virtual path components.
        for component in path.components() {
            current.push(component).unwrap();

            // Check if a virtual path is a mount point.
            if let Some(path) = mounts.get(&current) {
                directory = Directory {
                    path: path.clone(),
                    virtual_path: VpathBuf::new(),
                };
            } else {
                // Build a real path.
                let mut path = directory.path;

                path.push(component);

                // Get file metadata.
                let meta = match std::fs::metadata(&path) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            return None;
                        } else {
                            panic!("Cannot get the metadata of {}: {e}.", path.display());
                        }
                    }
                };

                // Check file type.
                if meta.is_file() {
                    return Some(Item::File(File {
                        path,
                        virtual_path: current,
                    }));
                }

                directory = Directory {
                    path,
                    virtual_path: VpathBuf::new(),
                };
            }
        }

        // If we reached here that mean the the last component is a directory.
        directory.virtual_path = current;

        Some(Item::Directory(directory))
    }

    pub fn mount<T, S>(&self, target: T, src: S) -> Result<(), MountError>
    where
        T: Into<VpathBuf>,
        S: Into<PathBuf>,
    {
        use std::collections::hash_map::Entry;

        let mut mounts = self.mounts.write().unwrap();

        match mounts.entry(target.into()) {
            Entry::Occupied(_) => return Err(MountError::AlreadyMounted),
            Entry::Vacant(e) => e.insert(src.into()),
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
    virtual_path: VpathBuf,
}

impl Directory {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn virtual_path(&self) -> &Vpath {
        &self.virtual_path
    }
}

/// A virtual file.
pub struct File {
    path: PathBuf,
    virtual_path: VpathBuf,
}

impl File {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn virtual_path(&self) -> &Vpath {
        &self.virtual_path
    }
}

/// Represents the errors for [`Fs::mount()`].
#[derive(Debug, Error)]
pub enum MountError {
    #[error("target is already mounted")]
    AlreadyMounted,
}
