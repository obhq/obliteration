use crate::errno::Errno;
use crate::fs::{VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use byteorder::{LittleEndian, WriteBytesExt};
use macros::vpath;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Dmem1 {}

impl Dmem1 {
    pub const PATH: &VPath = vpath!("/dev/dmem1");

    pub const COM10: u64 = 0x4008800a;
    pub const TOTAL_SIZE: usize = 6 * 1024 * 1024 * 1024; // 6 GiB

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
        com: u64,
        mut data: &mut [u8],
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match com {
            Self::COM10 => {
                data.write_u64::<LittleEndian>(Self::TOTAL_SIZE as u64)
                    .unwrap();
            }
            _ => todo!(),
        }

        Ok(())
    }
}

impl Display for Dmem1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
