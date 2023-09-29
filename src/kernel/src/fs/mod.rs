pub use self::item::*;
pub use self::path::*;

use crate::errno::{Errno, EACCES, ENOENT};
use gmtx::{GroupMutex, MutexGroup};
use param::Param;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs::File;
use std::num::NonZeroI32;
use std::path::PathBuf;
use thiserror::Error;

mod item;
mod path;

/// A virtual filesystem for emulating a PS4 filesystem.
#[derive(Debug)]
pub struct Fs {
    mounts: GroupMutex<HashMap<VPathBuf, MountSource>>,
    revoked: GroupMutex<HashSet<VPathBuf>>,
    app: VPathBuf,
}

impl Fs {
    pub fn new<P: Into<PathBuf>>(system: P, game: P) -> Self {
        let system = system.into();
        let game = game.into();
        let mut mounts: HashMap<VPathBuf, MountSource> = HashMap::new();

        // Mount rootfs.
        mounts.insert(VPathBuf::new(), MountSource::Host(system.clone()));

        // Get path to param.sfo.
        let mut path = game.join("sce_sys");

        path.push("param.sfo");

        // Open param.sfo.
        let param = match File::open(&path) {
            Ok(v) => v,
            Err(e) => panic!("Cannot open {}: {}.", path.display(), e),
        };

        // Load param.sfo.
        let param = match Param::read(param) {
            Ok(v) => v,
            Err(e) => panic!("Cannot read {}: {}.", path.display(), e),
        };

        // Create a directory for mounting PFS.
        let mut pfs = system.join("mnt");

        pfs.push("sandbox");
        pfs.push("pfsmnt");

        if let Err(e) = std::fs::create_dir_all(&pfs) {
            panic!("Cannot create {}: {}.", pfs.display(), e);
        }

        // Mount game directory.
        let pfs: VPathBuf = format!("/mnt/sandbox/pfsmnt/{}-app0-patch0-union", param.title_id())
            .try_into()
            .unwrap();

        mounts.insert(pfs.clone(), MountSource::Host(game));

        // Create a directory for mounting app0.
        let mut app = system.join("mnt");

        app.push("sandbox");
        app.push(format!("{}_000", param.title_id()));

        if let Err(e) = std::fs::create_dir_all(&app) {
            panic!("Cannot create {}: {}.", app.display(), e);
        }

        // Mount /mnt/sandbox/{id}_000/app0 to /mnt/sandbox/pfsmnt/{id}-app0-patch0-union.
        let mg = MutexGroup::new("fs");
        let app: VPathBuf = format!("/mnt/sandbox/{}_000", param.title_id())
            .try_into()
            .unwrap();

        mounts.insert(app.join("app0").unwrap(), MountSource::Bind(pfs));

        Self {
            mounts: mg.new_member(mounts),
            revoked: mg.new_member(HashSet::new()),
            app,
        }
    }

    pub fn app(&self) -> &VPath {
        self.app.borrow()
    }

    pub fn get(&self, path: &VPath) -> Result<FsItem, FsError> {
        // Resolve.
        let mounts = self.mounts.read();
        let item = match path.as_str() {
            "/dev/console" => FsItem::Device(VDev::Console),
            _ => Self::resolve(&mounts, path).ok_or(FsError::NotFound)?,
        };

        // Check if file has been revoked.
        if self.revoked.read().contains(item.vpath()) {
            return Err(FsError::Revoked);
        }

        Ok(item)
    }

    pub fn revoke<P: Into<VPathBuf>>(&self, path: P) {
        self.revoked.write().insert(path.into());
    }

    fn resolve(mounts: &HashMap<VPathBuf, MountSource>, path: &VPath) -> Option<FsItem> {
        let mut current = VPathBuf::new();
        let root = match mounts.get(&current).unwrap() {
            MountSource::Host(v) => v,
            MountSource::Bind(_) => unreachable!(),
        };

        // Walk on virtual path components.
        let mut directory = VDir::new(root.clone(), VPathBuf::new());

        for component in path.components() {
            current.push(component).unwrap();

            // Check if a virtual path is a mount point.
            if let Some(mount) = mounts.get(&current) {
                let path = match mount {
                    MountSource::Host(v) => v.to_owned(),
                    MountSource::Bind(v) => match Self::resolve(mounts, v)? {
                        FsItem::Directory(d) => d.into_path(),
                        _ => unreachable!(),
                    },
                };

                directory = VDir::new(path, VPathBuf::new());
            } else {
                // Build a real path.
                let mut path = directory.into_path();

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
                    return Some(FsItem::File(VFile::new(path, current)));
                }

                directory = VDir::new(path, VPathBuf::new());
            }
        }

        // If we reached here that mean the the last component is a directory.
        Some(FsItem::Directory(VDir::new(directory.into_path(), current)))
    }
}

/// Source of mount point.
#[derive(Debug)]
pub enum MountSource {
    Host(PathBuf),
    Bind(VPathBuf),
}

/// Represents an error when the operation of virtual filesystem is failed.
#[derive(Debug, Error)]
pub enum FsError {
    #[error("no such file or directory")]
    NotFound,

    #[error("the specified file has access revoked")]
    Revoked,
}

impl Errno for FsError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotFound => ENOENT,
            Self::Revoked => EACCES,
        }
    }
}
