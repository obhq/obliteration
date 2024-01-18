#![allow(dead_code, unused_variables)]

use crate::errno::{Errno, ENOTTY, ENXIO};
use crate::fs::{IoCmd, VFile, VFileOps};
use crate::process::VThread;
use std::num::NonZeroI32;
use thiserror::Error;

const BLOCKPOOL_FILEOPS: VFileOps = VFileOps {
    read: |_, _, _| Err(GenericError::InvalidOperation.into()),
    write: |_, _, _| Err(GenericError::InvalidOperation.into()),
    ioctl: blockpool_ioctl,
};

pub const UNK_CMD1: IoCmd = IoCmd::iowr::<(u64, u64, u64, u64)>(0xa8, 1);
pub const UNK_CMD2: IoCmd = IoCmd::ior::<(u32, u32, u32, u32)>(0xa8, 2);

fn blockpool_ioctl(
    file: &VFile,
    cmd: IoCmd,
    buf: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    match cmd {
        UNK_CMD1 => todo!("blockpool ioctl cmd 1"),
        UNK_CMD2 => todo!("blockpool ioctl cmd 2"),
        _ => Err(IoctlError::InvalidCommand(cmd).into()),
    }
}

#[derive(Debug, Error)]
pub enum GenericError {
    #[error("Invalid operation")]
    InvalidOperation,
}

impl Errno for GenericError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            GenericError::InvalidOperation => ENXIO,
        }
    }
}

#[derive(Debug, Error)]
pub enum IoctlError {
    #[error("Invalid command {0}")]
    InvalidCommand(IoCmd),
}

impl Errno for IoctlError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            IoctlError::InvalidCommand(_) => ENOTTY,
        }
    }
}
