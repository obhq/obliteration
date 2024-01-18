#![allow(dead_code, unused_variables)]

use crate::errno::{Errno, ENXIO};
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
pub const UNK_CMD2: IoCmd = IoCmd::ior::<(u64, u64)>(0xa8, 2);

fn blockpool_ioctl(
    file: &VFile,
    cmd: IoCmd,
    buf: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    todo!("blockpool_ioctl")
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
pub enum IoctlError {}

impl Errno for IoctlError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}
