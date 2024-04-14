use super::unix::UnixProtocol;
use super::{InetProtocol, Socket};
use crate::errno::Errno;
use crate::fs::IoCmd;
use crate::process::VThread;
use std::num::NonZeroI32;
use std::sync::Arc;

/// An implementation of the `pr_usrreqs` struct. This is subject to potential refactors, as it has to cover a lot of code
/// and therefore it is impossible to fully predict the correct implementation. In the future, this struct might end up containing functions from the
/// `protosw` struct as well.
pub(super) trait SocketBackend {
    fn attach(&self, socket: &Arc<Socket>, td: &VThread) -> Result<(), Box<dyn Errno>>;

    // TODO: a ifnet argument might have to be added in the future
    fn control(
        &self,
        socket: &Arc<Socket>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>>;

    fn listen(
        &self,
        socket: &Arc<Socket>,
        backlog: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>>;
}
#[repr(u8)]
#[derive(Debug)]
pub(super) enum Protocol {
    Unix(UnixProtocol) = 1,
    Inet(InetProtocol) = 2,
}

impl Protocol {
    pub fn find(domain: i32, ty: i32, proto: Option<NonZeroI32>) -> Option<Self> {
        let protocol = match domain {
            1 => {
                let protocol = match (ty, proto) {
                    (1, None) => UnixProtocol::Stream,
                    (2, None) => UnixProtocol::Datagram,
                    (5, None) => UnixProtocol::SeqPacket,
                    _ => return None,
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
