use super::{Socket, SocketBackend};
use crate::errno::Errno;
use crate::fs::IoCmd;
use crate::process::VThread;
use std::sync::Arc;

#[derive(Debug)]
pub(super) enum UnixProtocol {
    Stream = 1,
    Datagram = 2,
    SeqPacket = 5,
}

impl SocketBackend for UnixProtocol {
    fn attach(&self, _: &Arc<Socket>, _: &VThread) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn control(
        &self,
        _: &Arc<Socket>,
        _: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn listen(
        &self,
        _socket: &Arc<Socket>,
        _backlog: i32,
        _td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
