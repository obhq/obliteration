use super::{FsError, VFileOps, VPath, VPathBuf, dev::{dipsw::Dipsw, stdout::Stdout}};
use crate::fs::dev::console::Console;
use std::path::{Path, PathBuf};

/// An item in the virtual filesystem.
pub enum FsItem {
    Directory(HostDir),
    File(HostFile),
    Device(VDev),
}

impl FsItem {
    pub fn is_character(&self) -> bool {
        match self {
            Self::Device(d) => match d {
                VDev::Console => true,
                VDev::Dipsw => true,
                VDev::Stdout => true,
            },
            _ => false,
        }
    }

    pub fn vpath(&self) -> &VPath {
        match self {
            Self::Directory(v) => &v.vpath,
            Self::File(v) => &v.vpath,
            Self::Device(d) => match d {
                VDev::Console => Console::PATH,
                VDev::Dipsw => Dipsw::PATH,
                VDev::Stdout => Stdout::PATH,
            },
        }
    }

    pub fn open(&self) -> Result<Box<dyn VFileOps>, FsError> {
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
pub enum VDev {
    Console,
    Dipsw,
    Stdout
}

impl VDev {
    pub fn open(&self) -> Result<Box<dyn VFileOps>, FsError> {
        let ops: Box<dyn VFileOps> = match self {
            Self::Console => Box::new(Console::new()),
            Self::Dipsw => Box::new(Dipsw::new()),
            Self::Stdout => Box::new(Stdout::new()),
        };

        Ok(ops)
    }
}
