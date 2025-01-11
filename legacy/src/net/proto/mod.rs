use super::{SockAddr, Socket};
use crate::errno::{Errno, EOPNOTSUPP};
use crate::fs::IoCmd;
use crate::process::VThread;
use macros::Errno;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

use self::inet::*;
use self::unix::*;

mod inet;
mod unix;

/// An implementation of the `pr_usrreqs` struct. This is subject to potential refactors, as it has to cover a lot of code
/// and therefore it is impossible to fully predict the correct implementation. In the future, this struct might end up containing functions from the
/// `protosw` struct as well.
pub(super) trait SocketBackend {
    #[allow(unused_variables)]
    fn attach(&self, socket: &Arc<Socket>, td: &VThread) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(AttachError::NotSupported))
    }

    #[allow(unused_variables)]
    fn bind(
        &self,
        socket: &Arc<Socket>,
        addr: &SockAddr,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(AttachError::NotSupported))
    }

    #[allow(unused_variables)]
    fn connect(
        &self,
        socket: &Arc<Socket>,
        addr: &SockAddr,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(ConnectError::NotSupported))
    }

    // TODO: a ifnet argument might have to be added in the future
    #[allow(unused_variables)]
    fn control(
        &self,
        socket: &Arc<Socket>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(ControlError::NotSupported))
    }

    #[allow(unused_variables)]
    fn listen(
        &self,
        socket: &Arc<Socket>,
        backlog: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(ListenError::NotSupported))
    }
}
#[derive(Debug)]
pub(super) enum Protocol {
    Unix(UnixProtocol), // 1
    Inet(InetProtocol), // 2
}

impl Protocol {
    pub fn find(domain: i32, ty: i32, proto: Option<NonZeroI32>) -> Option<Self> {
        let protocol = match domain {
            1 => {
                let protocol = match (ty, proto) {
                    (1, None) => UnixProtocol::Stream,
                    (2, None) => UnixProtocol::Datagram,
                    (5, None) => UnixProtocol::SeqPacket,
                    _ => todo!(),
                };

                Protocol::Unix(protocol)
            }
            2 => {
                let protocol = match (ty, proto) {
                    (6, None) => InetProtocol::UdpPeerToPeer,
                    _ => todo!(),
                };
                Protocol::Inet(protocol)
            }
            _ => todo!(),
        };

        Some(protocol)
    }
}

impl SocketBackend for Protocol {
    fn attach(&self, socket: &Arc<Socket>, td: &VThread) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::Unix(protocol) => protocol.attach(socket, td),
            Self::Inet(protocol) => protocol.attach(socket, td),
        }
    }

    fn bind(
        &self,
        socket: &Arc<Socket>,
        addr: &SockAddr,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::Unix(protocol) => protocol.connect(socket, addr, td),
            Self::Inet(protocol) => protocol.connect(socket, addr, td),
        }
    }

    fn connect(
        &self,
        socket: &Arc<Socket>,
        addr: &SockAddr,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::Unix(protocol) => protocol.connect(socket, addr, td),
            Self::Inet(protocol) => protocol.connect(socket, addr, td),
        }
    }

    fn control(
        &self,
        socket: &Arc<Socket>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::Unix(protocol) => protocol.control(socket, cmd, td),
            Self::Inet(protocol) => protocol.control(socket, cmd, td),
        }
    }

    fn listen(
        &self,
        socket: &Arc<Socket>,
        backlog: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match self {
            Self::Unix(protocol) => protocol.listen(socket, backlog, td),
            Self::Inet(protocol) => protocol.listen(socket, backlog, td),
        }
    }
}

#[derive(Debug, Error, Errno)]
pub(super) enum AttachError {
    #[error("attaching is not supported for this protocol")]
    #[errno(EOPNOTSUPP)]
    NotSupported,
}

#[derive(Debug, Error, Errno)]
pub(super) enum BindError {
    #[error("binding is not supported for this protocol")]
    #[errno(EOPNOTSUPP)]
    NotSupported,
}

#[derive(Debug, Error, Errno)]
pub(super) enum ConnectError {
    #[error("connecting is not supported for this protocol")]
    #[errno(EOPNOTSUPP)]
    NotSupported,
}

#[derive(Debug, Error, Errno)]
pub(super) enum ControlError {
    #[error("controlling is not supported for this protocol")]
    #[errno(EOPNOTSUPP)]
    NotSupported,
}

#[derive(Debug, Error, Errno)]
pub(super) enum ListenError {
    #[error("listening is not supported for this protocol")]
    #[errno(EOPNOTSUPP)]
    NotSupported,
}
