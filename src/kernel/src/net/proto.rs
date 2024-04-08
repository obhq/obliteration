use super::{InetProtocol, Socket};
use crate::errno::Errno;
use crate::fs::IoCmd;
use crate::process::VThread;
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
}
#[repr(u8)]
#[derive(Debug)]
pub(super) enum Protocol {
    Inet(InetProtocol) = 2,
}

impl SocketBackend for Protocol {
    fn attach(&self, socket: &Arc<Socket>, td: &VThread) -> Result<(), Box<dyn Errno>> {
        match self {
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
            Self::Inet(protocol) => protocol.control(socket, cmd, td),
        }
    }
}
