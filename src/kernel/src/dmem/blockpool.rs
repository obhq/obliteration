use crate::errno::{Errno, ENOTTY};
use crate::fs::{FileBackend, IoCmd, Stat, VFile};
use crate::process::VThread;
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct BlockPool {}

impl FileBackend for BlockPool {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            BLOCKPOOL_CMD1 => todo!("blockpool ioctl cmd 1"),
            BLOCKPOOL_CMD2 => todo!("blockpool ioctl cmd 2"),
            _ => Err(IoctlError::InvalidCommand(cmd).into()),
        }
    }

    fn stat(self: &Arc<Self>, _: &VFile, _: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.block_size = 0x10000;
        stat.mode = 0o130000;

        todo!()
    }
}

#[derive(Debug, Error)]
pub enum IoctlError {
    #[error("invalid command {0}")]
    #[errno(ENOTTY)]
    InvalidCommand(IoCmd),
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
