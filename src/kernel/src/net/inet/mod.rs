use super::{Socket, SocketBackend};
use crate::errno::{Errno, EOPNOTSUPP};
use crate::fs::IoCmd;
use crate::process::VThread;
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub(super) enum InetProtocol {
    UdpPeerToPeer,
}

impl SocketBackend for InetProtocol {
    fn attach(&self, _: &Arc<Socket>, _: &VThread) -> Result<(), Box<dyn Errno>> {
        //TODO: properly implement this.
        match self {
            Self::UdpPeerToPeer => Ok(()),
        }
    }

    fn control(
        &self,
        _: &Arc<Socket>,
        cmd: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::UdpPeerToPeer => match cmd {
                // TODO: properly implement this. It is difficult to judge what it currently does,
                // because the socket is simply created, this ioctl is called and then the socket is immediately closed.
                IoCmd::BNETUNK(_) => Ok(()),
                _ => todo!(),
            },
        }
    }

    fn listen(
        &self,
        _socket: &Arc<Socket>,
        _backlog: i32,
        _td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::UdpPeerToPeer => Err(Box::new(ListenError::ListenError)),
        }
    }
}

#[derive(Debug, Error, Errno)]
enum ListenError {
    #[error("listening is not supported for this protocol")]
    #[errno(EOPNOTSUPP)]
    ListenError,
}
