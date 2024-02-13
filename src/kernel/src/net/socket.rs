use crate::errno::EAFNOSUPPORT;
use crate::fs::{FileBackend, IoCmd, VFile};
use crate::ucred::{PrisonAllow, PrisonFlags, Ucred};
use crate::{
    errno::{Errno, EPIPE},
    net::AddressFamily,
    process::VThread,
};
use bitflags::bitflags;
use std::{num::NonZeroI32, sync::Arc};
use thiserror::Error;

#[derive(Debug)]
pub struct Socket {
    ty: i32,                // so_type
    options: SocketOptions, // so_options
    cred: Arc<Ucred>,       // so_cred
    name: Option<Box<str>>,
}

impl Socket {
    /// See `socreate` on the PS4 for a reference.
    pub fn new(
        domain: i32,
        ty: i32,
        proto: i32,
        cred: &Arc<Ucred>,
        td: &VThread,
        name: Option<&str>,
    ) -> Result<Arc<Self>, SocketCreateError> {
        todo!()
    }

    fn options(&self) -> SocketOptions {
        self.options
    }

    /// See `sosend` on the PS4 for a reference.
    fn send(&self, buf: &[u8], td: Option<&VThread>) -> Result<usize, SendError> {
        todo!()
    }

    /// See `soreceive` on the PS4 for a reference.
    fn receive(&self, buf: &mut [u8], td: Option<&VThread>) -> Result<usize, ReceiveError> {
        todo!()
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct SocketOptions: i16 {
        const NOSIGPIPE = 0x0800;
    }
}

#[derive(Debug, Error)]
pub enum SocketCreateError {}

impl Errno for SocketCreateError {
    fn errno(&self) -> NonZeroI32 {
        match *self {}
    }
}

impl FileBackend for Socket {
    /// See soo_read on the PS4 for a reference.
    fn read(
        self: &Arc<Self>,
        _: &VFile,
        buf: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        let read = self.receive(buf, td)?;

        Ok(read)
    }

    fn write(
        self: &Arc<Self>,
        _: &VFile,
        buf: &[u8],
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
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

#[derive(Debug, Error)]
enum ReceiveError {}

impl Errno for ReceiveError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}

#[derive(Debug, Error)]
enum SendError {
    #[error("Broken pipe")]
    BrokenPipe,
}

impl Errno for SendError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::BrokenPipe => EPIPE,
        }
    }
}

impl Ucred {
    /// See `prison_check_af` on the PS4 for a reference.
    pub fn prison_check_address_family(
        &self,
        family: AddressFamily,
    ) -> Result<(), PrisonCheckAfError> {
        let pr = self.prison();

        match family {
            AddressFamily::UNIX | AddressFamily::ROUTE => {}
            AddressFamily::INET => todo!(),
            AddressFamily::INET6 => {
                if pr.flags().intersects(PrisonFlags::IP6) {
                    todo!()
                }
            }
            _ => {
                if !pr.allow().intersects(PrisonAllow::ALLOW_SOCKET_AF) {
                    return Err(PrisonCheckAfError::SocketAddressFamilyNotAllowed(family));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PrisonCheckAfError {
    #[error("the address family {0} is not allowed by prison")]
    SocketAddressFamilyNotAllowed(AddressFamily),
}

impl Errno for PrisonCheckAfError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::SocketAddressFamilyNotAllowed(_) => EAFNOSUPPORT,
        }
    }
}
