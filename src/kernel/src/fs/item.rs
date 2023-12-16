use super::{
    dev::{deci_tty6::DeciTty6, dipsw::Dipsw, dmem0::Dmem0, dmem1::Dmem1, dmem2::Dmem2},
    FsError, VFileOps, VPath, VPathBuf,
};
use crate::{fs::dev::console::Console, process::VProc};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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
                VDev::DeciTty6 => true,
                VDev::Dmem0 => true,
                VDev::Dmem1 => true,
                VDev::Dmem2 => true,
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
                VDev::DeciTty6 => DeciTty6::PATH,
                VDev::Dmem0 => Dmem0::PATH,
                VDev::Dmem1 => Dmem1::PATH,
                VDev::Dmem2 => Dmem2::PATH,
            },
        }
    }

    pub fn open(&self, vp: &Arc<VProc>) -> Result<Box<dyn VFileOps>, FsError> {
        match self {
            Self::Directory(_) => todo!("VFileOps for host directory"),
            Self::File(_) => todo!("VFileOps for host file"),
            Self::Device(d) => d.open(vp),
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
    DeciTty6,
    Dmem0,
    Dmem1,
    Dmem2,
}

impl VDev {
    pub fn open(&self, vp: &Arc<VProc>) -> Result<Box<dyn VFileOps>, FsError> {
        let ops: Box<dyn VFileOps> = match self {
            Self::Console => Box::new(Console::new()),
            Self::Dipsw => Box::new(Dipsw::new()),
            Self::DeciTty6 => Box::new(DeciTty6::new()),
            Self::Dmem0 => Box::new(Dmem0::new()),
            Self::Dmem1 => Box::new(Dmem1::new(vp)),
            Self::Dmem2 => Box::new(Dmem2::new()),
        };

        Ok(ops)
    }
}
