use crate::errno::Errno;
use crate::fs::{DefaultFileBackendError, FileBackend, IoCmd, PollEvents, Stat, VFile};
use crate::process::VThread;
use std::sync::Arc;

#[derive(Debug)]
pub struct BlockPool {}

impl FileBackend for BlockPool {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::BPOOLEXPAND(args) => todo!(),
            IoCmd::BPOOLSTATS(out) => todo!(),
            _ => Err(Box::new(DefaultFileBackendError::IoctlNotSupported)),
        }
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn poll(self: &Arc<Self>, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents {
        todo!()
    }

    fn stat(self: &Arc<Self>, _: &VFile, _: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.block_size = 0x10000;
        stat.mode = 0o130000;

        todo!()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct BlockpoolExpandArgs {
    len: usize,
    search_start: usize,
    search_end: usize,
    alignment: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct BlockpoolStats {
    avail_flushed: i32,
    avail_cached: i32,
    allocated_flushed: i32,
    allocated_cached: i32,
}
