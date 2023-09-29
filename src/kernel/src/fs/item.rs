use super::{VPath, VPathBuf};
use macros::vpath;
use std::path::{Path, PathBuf};

/// An item in the virtual filesystem.
pub enum FsItem {
    Directory(VDir),
    File(VFile),
    Device(VDev),
}

impl FsItem {
    pub fn is_character(&self) -> bool {
        match self {
            Self::Device(d) => match d {
                VDev::Console => true,
            },
            _ => false,
        }
    }

    pub fn vpath(&self) -> &VPath {
        match self {
            Self::Directory(v) => &v.vpath,
            Self::File(v) => &v.vpath,
            Self::Device(d) => match d {
                VDev::Console => vpath!("/dev/console"),
            },
        }
    }
}

/// A virtual directory.
pub struct VDir {
    path: PathBuf,
    vpath: VPathBuf,
}

impl VDir {
    pub(super) fn new(path: PathBuf, vpath: VPathBuf) -> Self {
        Self { path, vpath }
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

/// A virtual file.
pub struct VFile {
    path: PathBuf,
    vpath: VPathBuf,
}

impl VFile {
    pub(super) fn new(path: PathBuf, vpath: VPathBuf) -> Self {
        Self { path, vpath }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn vpath(&self) -> &VPath {
        &self.vpath
    }

    pub fn into_vpath(self) -> VPathBuf {
        self.vpath
    }
}

/// A virtual device.
pub enum VDev {
    Console,
}
