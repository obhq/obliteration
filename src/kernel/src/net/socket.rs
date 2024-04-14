use super::{GetOptError, Protocol, SetOptError, SockAddr, SockOpt, SocketBackend};
use crate::fs::{
    DefaultFileBackendError, FileBackend, IoCmd, PollEvents, Stat, TruncateLength, Uio, UioMut,
    VFile,
};
use crate::ucred::Ucred;
use crate::{
    errno::{Errno, EPIPE, EPROTONOSUPPORT},
    process::VThread,
};
use macros::Errno;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct Socket {
    cred: Arc<Ucred>, // so_cred
    name: Option<Box<str>>,
    backend: Protocol, // so_proto + so_type
}

impl Socket {
    #[allow(unused_variables)] // TODO: remove when implementing
    /// See `socreate` on the PS4 for a reference.
    pub fn new(
        domain: i32,
        ty: i32,
        proto: Option<NonZeroI32>,
        cred: &Arc<Ucred>,
        td: &VThread,
        name: Option<&str>,
    ) -> Result<Arc<Self>, SocketCreateError> {
        // TODO: implement prison_check_af
        let backend =
            Protocol::find(domain, ty, proto).ok_or(SocketCreateError::NoProtocolFound)?;

        let socket = Arc::new(Self {
            cred: Arc::clone(cred),
            name: name.map(|s| s.into()),
            backend,
        });

        socket
            .backend
            .attach(&socket, td)
            .map_err(SocketCreateError::AttachError)?;

        Ok(socket)
    }

    /// See `sosetopt` on the PS4 for a reference.
    #[allow(dead_code, unused_variables)] // TODO: remove when implementing
    fn setopt(&self, opt: SockOpt) -> Result<(), SetOptError> {
        todo!()
    }

    /// See `sogetopt` on the PS4 for a reference.
    #[allow(dead_code, unused_variables)] // TODO: remove when implementing
    fn getopt(&self, opt: SockOpt) -> Result<(), GetOptError> {
        todo!()
    }

    /// See `sosend` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: remove when implementing
    fn send(&self, buf: &mut Uio, td: Option<&VThread>) -> Result<usize, SendError> {
        todo!()
    }

    /// See `soreceive` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: remove when implementing
    fn receive(&self, buf: &mut UioMut, td: Option<&VThread>) -> Result<usize, ReceiveError> {
        todo!()
    }

    pub fn bind(self: &Arc<Self>, addr: &SockAddr, td: &VThread) -> Result<(), Box<dyn Errno>> {
        self.backend.bind(self, addr, td)?;

        Ok(())
    }

    /// See `soconnect` on the PS4 for a reference.
    pub fn connect(self: &Arc<Self>, addr: &SockAddr, td: &VThread) -> Result<(), Box<dyn Errno>> {
        self.backend.connect(self, addr, td)?;

        Ok(())
    }

    /// See `solisten` on the PS4 for a reference.
    pub fn listen(self: &Arc<Self>, backlog: i32, td: Option<&VThread>) -> Result<(), ListenError> {
        self.backend.listen(self, backlog, td)?;

        Ok(())
    }
}

impl FileBackend for Socket {
    #[allow(unused_variables)] // TODO: remove when implementing
    /// See soo_read on the PS4 for a reference.
    fn read(
        self: &Arc<Self>,
        _: &VFile,
        buf: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    /// See soo_write on the PS4 for a reference.
    fn write(
        self: &Arc<Self>,
        _: &VFile,
        buf: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::FIONBIO(_) => todo!("socket ioctl with FIONBIO"),
            IoCmd::FIOASYNC(_) => todo!("socket ioctl with FIOASYNC"),
            IoCmd::FIONREAD(_) => todo!("socket ioctl with FIONREAD"),
            IoCmd::FIONWRITE(_) => todo!("socket ioctl with FIONWRITE"),
            IoCmd::FIONSPACE(_) => todo!("socket ioctl with FIONSPACE"),
            IoCmd::FIOSETOWN(_) => todo!("socket ioctl with FIOSETOWN"),
            IoCmd::FIOGETOWN(_) => todo!("socket ioctl with FIOGETOWN"),
            IoCmd::SIOCSPGRP(_) => todo!("socket ioctl with SIOCSPGRP"),
            IoCmd::SIOCGPGRP(_) => todo!("socket ioctl with SIOCGPGRP"),
            IoCmd::SIOCATMARK(_) => todo!("socket ioctl with SIOCATMARK"),
            _ => self.backend.control(self, cmd, td),
        }
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn poll(self: &Arc<Self>, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn stat(self: &Arc<Self>, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        todo!()
    }

    fn truncate(
        self: &Arc<Self>,
        _: &VFile,
        _: TruncateLength,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::InvalidValue))
    }
}

#[derive(Debug, Error, Errno)]
pub enum SocketCreateError {
    #[error("no protocol found")]
    #[errno(EPROTONOSUPPORT)]
    NoProtocolFound,

    #[error("couldn't attach socket")]
    AttachError(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
enum ReceiveError {}

#[derive(Debug, Error, Errno)]
enum SendError {
    #[error("Broken pipe")]
    #[errno(EPIPE)]
    BrokenPipe,
}

#[derive(Debug, Error, Errno)]
pub enum ListenError {
    #[error("listen failed")]
    ListenFailed(#[from] Box<dyn Errno>),
}
