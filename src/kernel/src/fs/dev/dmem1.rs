use crate::errno::Errno;
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Dmem1 {}

impl Dmem1 {
    pub const PATH: &VPath = vpath!("/dev/dmem1");

    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for Dmem1 {
    fn write(&self, _: &VFile, _: &[u8], _: &Ucred, _: &VThread) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _: &crate::fs::VFile,
        _com: IoctlCom,
        _: &mut [u8],
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

impl Display for Dmem1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
