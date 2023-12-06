use crate::errno::Errno;
use crate::fs::{VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Dmem2 {}

impl Dmem2 {
    pub const PATH: &VPath = vpath!("/dev/dmem2");

    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for Dmem2 {
    fn write(&self, _: &VFile, _: &[u8], _: &Ucred, _: &VThread) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _: &crate::fs::VFile,
        _: u64,
        _: &mut [u8],
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

impl Display for Dmem2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
