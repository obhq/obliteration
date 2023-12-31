use crate::errno::{Errno, EPERM};
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::{VProc, VThread};
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct Dmem1 {
    total_size: usize,
    number: usize,
}

impl Dmem1 {
    pub const PATH: &'static VPath = vpath!("/dev/dmem1");

    pub const DMEM_GRP: u8 = 0x80;

    pub const COM10: IoctlCom = IoctlCom::ior::<usize>(Self::DMEM_GRP, 0xa);

    pub fn new() -> Self {
        Self {
            total_size: 0x13C_000_000, // TODO figure out the real value
            number: 1,
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
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        if cred.is_unk1() || cred.is_unk2() {
            return Err(Box::new(IoctlErr::BadCredentials));
        }

        if self.number != 2 && self.number != td.proc().dmem_container() && !cred.is_system() {
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
