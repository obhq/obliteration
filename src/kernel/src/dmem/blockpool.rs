use crate::errno::Errno;
use crate::fs::{FileBackend, IoCmd, Stat, VFile};
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
        todo!()
    }

    fn stat(self: &Arc<Self>, _: &VFile, _: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.block_size = 0x10000;
        stat.mode = 0o130000;

        todo!()
    }
}
