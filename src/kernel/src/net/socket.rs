use crate::fs::{DefaultError, FileBackend, IoCmd, Stat, TruncateLength, Uio, UioMut, VFile};
use crate::ucred::Ucred;
use crate::{
    errno::{Errno, EPIPE},
    process::VThread,
};
use bitflags::bitflags;
use macros::Errno;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

use super::{GetOptError, SetOptError, SockOpt};

#[derive(Debug)]
pub struct Socket {
    ty: i32,                // so_type
    options: SocketOptions, // so_options
    cred: Arc<Ucred>,       // so_cred
    name: Option<Box<str>>,
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
        todo!()
    }

    fn options(&self) -> SocketOptions {
        self.options
    }

    /// See `sosetopt` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: remove when implementing
    fn setopt(&self, opt: SockOpt) -> Result<(), SetOptError> {
        todo!()
    }

    /// See `sogetopt` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: remove when implementing
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
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct SocketOptions: i16 {
        const NOSIGPIPE = 0x0800;
    }
}

impl FileBackend for Socket {
    /// See soo_read on the PS4 for a reference.
    fn read(
        self: &Arc<Self>,
        _: &VFile,
        buf: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        let read = self.receive(buf, td)?;

        Ok(read)
    }

    fn write(
        self: &Arc<Self>,
        _: &VFile,
        buf: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        let written = match self.send(buf, td) {
            Ok(written) => written,
            Err(SendError::BrokenPipe) if self.options().intersects(SocketOptions::NOSIGPIPE) => {
                todo!()
            }
            Err(e) => return Err(e.into()),
        };

        Ok(written)
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
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
        Err(Box::new(DefaultError::InvalidValue))
    }
}

#[derive(Debug, Error, Errno)]
pub enum SocketCreateError {}

#[derive(Debug, Error, Errno)]
enum ReceiveError {}

#[derive(Debug, Error, Errno)]
enum SendError {
    #[error("Broken pipe")]
    #[errno(EPIPE)]
    BrokenPipe,
}
