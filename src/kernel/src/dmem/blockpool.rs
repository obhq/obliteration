use crate::errno::AsErrno;
use crate::fs::{FileBackend, IoCmd, PollEvents, Stat, VFile};
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
    ) -> Result<(), Box<dyn AsErrno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn poll(self: &Arc<Self>, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents {
        todo!()
    }

    fn stat(self: &Arc<Self>, _: &VFile, _: Option<&VThread>) -> Result<Stat, Box<dyn AsErrno>> {
        let mut stat = Stat::zeroed();

        stat.block_size = 0x10000;
        stat.mode = 0o130000;

        todo!()
    }
}
