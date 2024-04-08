use super::{Socket, SocketBackend};
use crate::errno::Errno;
use crate::fs::IoCmd;
use crate::process::VThread;
use std::sync::Arc;

#[derive(Debug)]
pub(super) enum InetProtocol {
    UdpP2P,
}

impl SocketBackend for InetProtocol {
    fn attach(&self, _: &Arc<Socket>, _: &VThread) -> Result<(), Box<dyn Errno>> {
        //TODO: properly implement this.
        Ok(())
    }

    fn control(
        &self,
        _: &Arc<Socket>,
        cmd: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::UdpP2P => match cmd {
                // TODO: properly implement this. It is difficult to judge what it currently does,
                // because the socket is simply created, this ioctl is called and then the socket is immediately closed.
                IoCmd::BNETUNK(_) => Ok(()),
                _ => todo!(),
            },
        }
    }
}
