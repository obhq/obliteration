use crate::errno::Errno;
use crate::fs::{VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Dmem0 {}

impl Dmem0 {
    pub const PATH: &VPath = vpath!("/dev/dmem0");

    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for Dmem0 {
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

impl Display for Dmem0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
