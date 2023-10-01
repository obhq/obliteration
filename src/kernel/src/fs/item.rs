use super::{Fs, FsError, VFileOps, VPath, VPathBuf};
use crate::console::Console;
use macros::vpath;
use std::path::{Path, PathBuf};

/// An item in the virtual filesystem.
pub enum FsItem<'a> {
    Directory(HostDir),
    File(HostFile),
    Device(VDev<'a>),
}

impl<'a> FsItem<'a> {
    pub fn is_character(&self) -> bool {
        match self {
            Self::Device(d) => match d {
                VDev::Console(_) => true,
            },
            _ => false,
        }
    }

    pub fn vpath(&self) -> &VPath {
        match self {
            Self::Directory(v) => &v.vpath,
            Self::File(v) => &v.vpath,
            Self::Device(d) => match d {
                VDev::Console(_) => vpath!("/dev/console"),
            },
        }
    }

    pub fn open(&self) -> Result<Box<dyn VFileOps + 'a>, FsError> {
        match self {
            Self::Directory(_) => todo!("VFileOps for host directory"),
            Self::File(_) => todo!("VFileOps for host file"),
            Self::Device(d) => d.open(),
        }
    }
}

/// A virtual directory backed by a real directory on the host.
pub struct HostDir {
    path: PathBuf,
    vpath: VPathBuf,
}

impl HostDir {
    pub(super) fn new(path: PathBuf, vpath: VPathBuf) -> Self {
        Self { path, vpath }
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

/// A virtual file backed by a real file on the host.
pub struct HostFile {
    path: PathBuf,
    vpath: VPathBuf,
}

impl HostFile {
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
pub enum VDev<'a> {
    Console(&'a Fs),
}

impl<'a> VDev<'a> {
    pub fn open(&self) -> Result<Box<dyn VFileOps + 'a>, FsError> {
        let ops = match self {
            Self::Console(_) => Box::new(Console::new()),
        };

        Ok(ops)
    }
}
