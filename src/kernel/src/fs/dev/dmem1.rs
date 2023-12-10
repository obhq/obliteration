use crate::errno::{Errno, EPERM};
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;
use thiserror::Error;

#[derive(Debug)]
pub struct Dmem1 {
    total_size: usize,
}

impl Dmem1 {
    pub const PATH: &VPath = vpath!("/dev/dmem1");

    pub const COM10: IoctlCom = IoctlCom::ior::<usize>(88, 0xa);

    pub fn new() -> Self {
        Self {
            total_size: 0x180_000_000, // 6 GB
        }
    }
}

impl VFileOps for Dmem1 {
    fn write(&self, _: &VFile, _: &[u8], _: &Ucred, _: &VThread) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _: &crate::fs::VFile,
        com: IoctlCom,
        data: &mut [u8],
        cred: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        if cred.is_unk1() || cred.is_unk2() {
            return Err(Box::new(IoctlErr::BadCredentials));
        }

        match com {
            Self::COM10 => {
                data.copy_from_slice(&self.total_size.to_ne_bytes());
            }
            _ => todo!("dmem1 ioctl with com = ({com})"),
        }

        Ok(())
    }
}

impl Display for Dmem1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}

#[derive(Error, Debug)]
enum IoctlErr {
    #[error("bad credentials")]
    BadCredentials,
}

impl Errno for IoctlErr {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::BadCredentials => EPERM,
        }
    }
}
