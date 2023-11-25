use crate::errno::Errno;
use crate::fs::{VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};

/// An implementation of `/dev/dipsw`.
#[derive(Debug)]
pub struct Dipsw {}

impl Dipsw {
    pub const PATH: &VPath = vpath!("/dev/dipsw");

    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for Dipsw {
    fn write(
        &self,
        _file: &VFile,
        _data: &[u8],
        _cred: &Ucred,
        _td: &VThread,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _file: &VFile,
        _com: u64,
        _data: &[u8],
        _cred: &Ucred,
        _td: &VThread,
    ) -> Result<Option<&[u8]>, Box<dyn Errno>> {
        todo!()
    }
}

impl Display for Dipsw {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
