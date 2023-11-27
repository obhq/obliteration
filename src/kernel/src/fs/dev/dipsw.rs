use crate::errno::{Errno, EINVAL, ENOENT};
use crate::fs::{VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::syscalls::SysErr;
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
        com: u64,
        data: *mut (),
        _: &Ucred,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match com {
            0x40048806 => {
                //todo return the correct value if unk_func1() = false and
                // unk_func2() = true
                let data = data as *mut u32;

                unsafe { *data = false as u32 };
            }
            0x40048807 => todo!("dipsw ioctl 0x40048807"),
            0x40088808 => todo!("dipsw ioctl 0x40088808"),
            0x40088809 => todo!("dipsw ioctl 0x40088809"),
            0x80028801 => todo!("dipsw ioctl 0x80028801"),
            0x80028802 => todo!("dipsw ioctl 0x80028802"),
            0x80108804 => todo!("dipsw ioctl 0x80108804"),
            0x80108805 => todo!("dipsw ioctl 0x80108805"),
            0x8010880a => todo!("dipsw ioctl 0x8010880a"),
            0xc0088803 => todo!("dipsw ioctl 0xc0088803"),
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
