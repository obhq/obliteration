use crate::errno::{Errno, ENOTTY, ENXIO};
use crate::fs::{IoCmd, Stat, VFile, VFileOps};
use crate::process::VThread;
use std::num::NonZeroI32;
use thiserror::Error;

const BLOCKPOOL_FILEOPS: VFileOps = VFileOps {
    read: |_, _, _| Err(GenericError::OperationNotSupported)?,
    write: |_, _, _| Err(GenericError::OperationNotSupported)?,
    ioctl: blockpool_ioctl,
    stat: blockpool_stat,
};

#[allow(dead_code)] // Remove this when blockpools are being implemented
fn blockpool_ioctl(
    file: &VFile,
    cmd: IoCmd,
    buf: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    match cmd {
        BLOCKPOOL_CMD1 => todo!("blockpool ioctl cmd 1"),
        BLOCKPOOL_CMD2 => todo!("blockpool ioctl cmd 2"),
        _ => Err(IoctlError::InvalidCommand(cmd).into()),
    }
}

#[allow(dead_code)] // Remove this when blockpools are being implemented
fn blockpool_stat(file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
    todo!()
}

#[derive(Debug, Error)]
pub enum GenericError {
    #[error("operation not supported")]
    OperationNotSupported,
}

impl Errno for GenericError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            GenericError::OperationNotSupported => ENXIO,
        }
    }
}

#[derive(Debug, Error)]
pub enum IoctlError {
    #[error("invalid command {0}")]
    InvalidCommand(IoCmd),
}

impl Errno for IoctlError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            IoctlError::InvalidCommand(_) => ENOTTY,
        }
    }
}

pub const BLOCKPOOL_CMD1: IoCmd = IoCmd::iowr::<BlockpoolCmd1Arg>(0xa8, 1);
pub const BLOCKPOOL_CMD2: IoCmd = IoCmd::ior::<BlockpoolCmd2Arg>(0xa8, 2);

#[repr(C)]
struct BlockpoolCmd1Arg {
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
}

#[repr(C)]
struct BlockpoolCmd2Arg {
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
}
