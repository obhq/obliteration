use self::path::{VPath, VPathBuf};
use param::Param;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use thiserror::Error;

pub mod path;

/// A virtual filesystem for emulating a PS4 filesystem.
pub struct Fs {
    mounts: RwLock<HashMap<VPathBuf, MountSource>>,
    app: VPathBuf,
}

impl Fs {
    pub fn new<P: Into<PathBuf>>(system: P, game: P) -> Result<Self, FsError> {
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
            Err(e) => return Err(FsError::OpenParamFailed(path, e)),
        };

        // Load param.sfo.
        let param = match Param::read(param) {
            Ok(v) => v,
            Err(e) => return Err(FsError::ReadParamFailed(path, e)),
        };

        // Create a directory for mounting PFS.
        let mut pfs = system.join("mnt");

        pfs.push("sandbox");
        pfs.push("pfsmnt");

        if let Err(e) = std::fs::create_dir_all(&pfs) {
            return Err(FsError::CreateDirectoryFailed(pfs, e));
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
            return Err(FsError::CreateDirectoryFailed(app, e));
        }

        // Mount /mnt/sandbox/{id}_000/app0 to /mnt/sandbox/pfsmnt/{id}-app0-patch0-union.
        let app: VPathBuf = format!("/mnt/sandbox/{}_000", param.title_id())
            .try_into()
            .unwrap();

        mounts.insert(app.join("app0").unwrap(), MountSource::Bind(pfs));

        Ok(Self {
            mounts: RwLock::new(mounts),
            app,
        })
    }

    pub fn app(&self) -> &VPath {
        self.app.borrow()
    }

    pub fn get_file(&self, path: &VPath) -> Option<VFile> {
        self.get(path).and_then(|i| match i {
            FsItem::Directory(_) => None,
            FsItem::File(v) => Some(v),
        })
    }

    pub fn get(&self, path: &VPath) -> Option<FsItem> {
        let mounts = self.mounts.read().unwrap();

        Self::resolve(&mounts, path)
    }

    fn resolve(mounts: &HashMap<VPathBuf, MountSource>, path: &VPath) -> Option<FsItem> {
        let mut current = VPathBuf::new();
        let root = match mounts.get(&current).unwrap() {
            MountSource::Host(v) => v,
            MountSource::Bind(_) => unreachable!(),
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
            if let Some(mount) = mounts.get(&current) {
                let path = match mount {
                    MountSource::Host(v) => v.to_owned(),
                    MountSource::Bind(v) => match Self::resolve(mounts, v)? {
                        FsItem::Directory(d) => d.path,
                        FsItem::File(f) => unreachable!(
                            "{} expected to be a directory but a file at {}.",
                            v,
                            f.path.display()
                        ),
                    },
                };

                directory = VDir {
                    path,
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
}

/// Source of mount point.
pub enum MountSource {
    Host(PathBuf),
    Bind(VPathBuf),
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

/// Represents the error for FS initialization.
#[derive(Debug, Error)]
pub enum FsError {
    #[error("cannot open {0}")]
    OpenParamFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot read {0}")]
    ReadParamFailed(PathBuf, #[source] param::ReadError),

    #[error("cannot create {0}")]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),
}
