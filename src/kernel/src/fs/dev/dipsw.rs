use crate::errno::{Errno, EINVAL};
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use byteorder::{LittleEndian, WriteBytesExt};
use macros::vpath;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;
use thiserror::Error;

/// An implementation of `/dev/dipsw`.
#[derive(Debug)]
pub struct Dipsw {}

impl Dipsw {
    pub const PATH: &VPath = vpath!("/dev/dipsw");

    pub fn new() -> Self {
        Self {}
    }
}

const COM1: IoctlCom = IoctlCom::iow::<i16>(0x88, 1); //TODO: figure out actual type
const COM2: IoctlCom = IoctlCom::iow::<i16>(0x88, 2); //TODO: figure out actual type
const COM3: IoctlCom = IoctlCom::iowr::<i16>(0x88, 3); //TODO: figure out actual type
const COM4: IoctlCom = IoctlCom::iow::<(i64, i64)>(0x88, 4); //TODO: figure out actual type, probably a struct
const COM5: IoctlCom = IoctlCom::iow::<(i64, i64)>(0x88, 5); //TODO: figure out actual type, probably a struct
const COM6: IoctlCom = IoctlCom::ior::<i32>(0x88, 6);
const COM7: IoctlCom = IoctlCom::ior::<i32>(0x88, 7); //TODO: figure out actual type
const COM8: IoctlCom = IoctlCom::ior::<i64>(0x88, 8); //TODO: figure out actual type
const COM9: IoctlCom = IoctlCom::ior::<i64>(0x88, 9); //TODO: figure out actual type
const COM10: IoctlCom = IoctlCom::iow::<(i64, i64)>(0x88, 10); //TODO: figure out actual type, probably a struct

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
        mut data: &mut [u8],
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match com {
            COM1 => todo!("dipsw ioctl 0x80028801"),
            COM2 => todo!("dipsw ioctl 0x80028802"),
            COM3 => todo!("dipsw ioctl 0xc0088803"),
            COM4 => todo!("dipsw ioctl 0x80108804"),
            COM5 => todo!("dipsw ioctl 0x80108805"),
            COM6 => {
                //todo write the correct value if unk_func1() = false and
                // unk_func2() = true
                data.write_i32::<LittleEndian>(false as i32).unwrap();
            }
            COM7 => todo!("dipsw ioctl 0x40048807"),
            COM8 => todo!("dipsw ioctl 0x40088808"),
            COM9 => todo!("dipsw ioctl 0x40088809"),
            COM10 => todo!("dipsw ioctl 0x8010880a"),
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
    #[error("invalid command passed to Dipsw::ioctl")]
    InvalidCommand,
}

impl Errno for IoctlErr {
    fn errno(&self) -> NonZeroI32 {
        match self {
            IoctlErr::InvalidCommand => EINVAL,
        }
    }
}
