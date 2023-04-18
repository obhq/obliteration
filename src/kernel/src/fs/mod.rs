use self::path::{VPath, VPathBuf};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use thiserror::Error;

pub mod path;

/// A virtual filesystem for emulating a PS4 filesystem.
pub struct Fs {
    mounts: RwLock<HashMap<VPathBuf, PathBuf>>,
}

impl Fs {
    pub(super) fn new() -> Self {
        Self {
            mounts: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &VPath) -> Option<FsItem> {
        // Get root mount point.
        let mut current = VPathBuf::new();

        let mounts = self.mounts.read().unwrap();
        let root = match mounts.get(&current) {
            Some(v) => v,
            None => panic!("No rootfs is mounted."),
        };

        // Open a root directory.
        let mut directory = VDir {
            path: root.clone(),
            virtual_path: VPathBuf::new(),
        };

        // Walk on virtual path components.
        for component in path.components() {
            current.push(component).unwrap();

            // Check if a virtual path is a mount point.
            if let Some(path) = mounts.get(&current) {
                directory = VDir {
                    path: path.clone(),
                    virtual_path: VPathBuf::new(),
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
                    return Some(FsItem::File(VFile {
                        path,
                        virtual_path: current,
                    }));
                }

                directory = VDir {
                    path,
                    virtual_path: VPathBuf::new(),
                };
            }
        }

        // If we reached here that mean the the last component is a directory.
        directory.virtual_path = current;

        Some(FsItem::Directory(directory))
    }

    pub fn mount<T, S>(&self, target: T, src: S) -> Result<(), MountError>
    where
        T: Into<VPathBuf>,
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
pub enum FsItem {
    Directory(VDir),
    File(VFile),
}

/// A virtual directory.
pub struct VDir {
    path: PathBuf,
    virtual_path: VPathBuf,
}

impl VDir {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn virtual_path(&self) -> &VPath {
        &self.virtual_path
    }
}

/// A virtual file.
pub struct VFile {
    path: PathBuf,
    virtual_path: VPathBuf,
}

impl VFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn virtual_path(&self) -> &VPath {
        &self.virtual_path
    }
}

/// Represents the errors for [`Fs::mount()`].
#[derive(Debug, Error)]
pub enum MountError {
    #[error("target is already mounted")]
    AlreadyMounted,
}
