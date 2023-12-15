use crate::errno::{Errno, EINVAL};
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;
use thiserror::Error;

/// An implementation of `/dev/dipsw`.
#[derive(Debug)]
pub struct Dipsw {}

impl Dipsw {
    pub const PATH: &VPath = vpath!("/dev/dipsw");

    pub const DIPSW_GRP: u8 = 0x88;

    const COM1: IoctlCom = IoctlCom::iow::<i16>(Self::DIPSW_GRP, 1); //TODO: figure out actual type
    const COM2: IoctlCom = IoctlCom::iow::<i16>(Self::DIPSW_GRP, 2); //TODO: figure out actual type
    const COM3: IoctlCom = IoctlCom::iowr::<i64>(Self::DIPSW_GRP, 3); //TODO: figure out actual type
    const COM4: IoctlCom = IoctlCom::iow::<(i64, i64)>(Self::DIPSW_GRP, 4); //TODO: figure out actual type, probably a struct
    const COM5: IoctlCom = IoctlCom::iow::<(i64, i64)>(Self::DIPSW_GRP, 5); //TODO: figure out actual type, probably a struct
    const COM6: IoctlCom = IoctlCom::ior::<i32>(Self::DIPSW_GRP, 6);
    const COM7: IoctlCom = IoctlCom::ior::<i32>(Self::DIPSW_GRP, 7); //TODO: figure out actual type
    const COM8: IoctlCom = IoctlCom::ior::<i64>(Self::DIPSW_GRP, 8); //TODO: figure out actual type
    const COM9: IoctlCom = IoctlCom::ior::<i64>(Self::DIPSW_GRP, 9); //TODO: figure out actual type
    const COM10: IoctlCom = IoctlCom::iow::<(i64, i64)>(Self::DIPSW_GRP, 10); //TODO: figure out actual type, probably a struct

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
        _: &VFile,
        com: IoctlCom,
        data: &mut [u8],
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match com {
            Self::COM1 => todo!("dipsw ioctl 0x80028801"),
            Self::COM2 => todo!("dipsw ioctl 0x80028802"),
            Self::COM3 => todo!("dipsw ioctl 0xc0088803"),
            Self::COM4 => todo!("dipsw ioctl 0x80108804"),
            Self::COM5 => todo!("dipsw ioctl 0x80108805"),
            Self::COM6 => {
                //todo write the correct value if unk_func1() = false and
                // unk_func2() = true
                data.copy_from_slice(&(false as i32).to_le_bytes());
            }
            Self::COM7 => todo!("dipsw ioctl 0x40048807"),
            Self::COM8 => todo!("dipsw ioctl 0x40088808"),
            Self::COM9 => todo!("dipsw ioctl 0x40088809"),
            Self::COM10 => todo!("dipsw ioctl 0x8010880a"),
            _ => return Err(Box::new(IoctlErr::InvalidCommand)),
        }

        Ok(())
    }
}

impl Display for Dipsw {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}

#[derive(Error, Debug)]
enum IoctlErr {
    #[error("invalid command passed to dipsw ioctl")]
    InvalidCommand,
}

impl Errno for IoctlErr {
    fn errno(&self) -> NonZeroI32 {
        match self {
            IoctlErr::InvalidCommand => EINVAL,
        }
    }
}
