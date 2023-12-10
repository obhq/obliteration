use crate::errno::Errno;
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct DeciTty6 {}

impl DeciTty6 {
    pub const PATH: &VPath = vpath!("/dev/deci_tty6");

    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for DeciTty6 {
    fn write(&self, _: &VFile, _: &[u8], _: &Ucred, _: &VThread) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _: &crate::fs::VFile,
        _: IoctlCom,
        _: &mut [u8],
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

impl Display for DeciTty6 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
