#![allow(dead_code, unused_variables)]
use crate::{
    errno::{Errno, EPERM, EPIPE, EPROTONOSUPPORT, EPROTOTYPE},
    net::{AddressFamily, Protosw},
    process::VThread,
    ucred::{PrisonCheckAfError, Ucred},
};
use bitflags::bitflags;
use std::{num::NonZeroI32, sync::Arc};
use thiserror::Error;

use super::{
    IoCmd, VFile, VFileOps, FIOASYNC, FIOGETOWN, FIONBIO, FIONREAD, FIONSPACE, FIONWRITE, FIOSETOWN,
};

#[derive(Debug)]
pub struct Socket {
    ty: i32,                 // so_type
    options: SocketOptions,  // so_options
    proto: &'static Protosw, // so_proto
    fibnum: i32,             // so_fibnum
    cred: Arc<Ucred>,        // so_cred
    pid: NonZeroI32,         // so_pid
    name: [u8; 32],
}

impl Socket {
    /// See `socreate` on the PS4 for a reference.
    pub(super) fn new(
        domain: i32,
        ty: i32,
        proto: i32,
        cred: &Arc<Ucred>,
        td: &VThread,
    ) -> Result<Arc<Self>, SocketCreateError> {
        if domain == 28 {
            return Err(SocketCreateError::IPv6NotSupported);
        }

        if !td.cred().is_system() {
            return Err(SocketCreateError::InsufficientCredentials);
        }

        if ty == 6 || ty == 10 {
            return Err(SocketCreateError::UnsupportedType);
        }

        let prp = if proto == 0 {
            Protosw::find_by_type(domain, ty)
        } else {
            Protosw::find_by_proto(domain, proto, ty)
        };

        let prp = prp.ok_or(SocketCreateError::NoProtocolSwitch)?;

        cred.prison_check_address_family(prp.domain().family())?;

        if prp.ty() != ty {
            return Err(SocketCreateError::WrongProtocolTypeForSocket);
        }

        let fibnum = match prp.domain().family() {
            AddressFamily::INET | AddressFamily::INET6 | AddressFamily::ROUTE => td.proc().fibnum(),
            _ => 0,
        };

        let so = Self {
            ty,
            options: SocketOptions::empty(),
            proto: prp,
            fibnum,
            cred: Arc::clone(cred),
            pid: td.proc().id(),
            name: [0; 32],
        };

        Ok(Arc::new(so))
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
pub(super) enum SocketCreateError {
    #[error("IPv6 is not supported")]
    IPv6NotSupported,

    #[error("Insufficient credentials")]
    InsufficientCredentials,

    #[error("Unsupported type")]
    UnsupportedType,

    #[error("Couldn't find protocol switch")]
    NoProtocolSwitch,

    #[error("Address family not allowed by prison")]
    PrisonCheckAfError(#[from] PrisonCheckAfError),

    #[error("Wrong protocol type for socket")]
    WrongProtocolTypeForSocket,
}

impl Errno for SocketCreateError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::IPv6NotSupported => todo!(),
            Self::InsufficientCredentials => EPERM,
            Self::UnsupportedType => EPROTONOSUPPORT,
            Self::NoProtocolSwitch => EPROTONOSUPPORT,
            Self::PrisonCheckAfError(e) => EPROTONOSUPPORT,
            Self::WrongProtocolTypeForSocket => EPROTOTYPE,
        }
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

pub const SOCKET_FILEOPS: VFileOps = VFileOps {
    //read: socket_read,
    write: socket_write,
    ioctl: socket_ioctl,
};

fn socket_read(
    file: &VFile,
    buf: &mut [u8],
    td: Option<&VThread>,
) -> Result<usize, Box<dyn Errno>> {
    let so = file.data_as_socket().unwrap();

    let read = so.receive(buf, td)?;

    Ok(read)
}

fn socket_write(file: &VFile, buf: &[u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
    let so = file.data_as_socket().unwrap();

    let written = match so.send(buf, td) {
        Ok(written) => written,
        Err(SendError::BrokenPipe) if so.options().intersects(SocketOptions::NOSIGPIPE) => todo!(),
        Err(e) => return Err(e.into()),
    };

    Ok(written)
}

fn socket_ioctl(
    file: &VFile,
    cmd: IoCmd,
    data: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    let so = file.data_as_socket().unwrap();

    match cmd {
        FIONBIO => todo!(),
        FIOASYNC => todo!(),
        FIONREAD => todo!(),
        FIONWRITE => todo!(),
        FIONSPACE => todo!(),
        FIOSETOWN => todo!(),
        FIOGETOWN => todo!(),
        SIOCSPGRP => todo!(),
        SIOCGPGRP => todo!(),
        SIOCATMARK => todo!(),
        cmd => match cmd.group() {
            b'i' => todo!(),
            b'r' => todo!(),
            _ => todo!(),
        },
    }
}

const SOCKET_GROUP: u8 = b's';

const SIOCSHIWAT: IoCmd = IoCmd::iow::<i32>(SOCKET_GROUP, 0);
const SIOCGHIWAT: IoCmd = IoCmd::ior::<i32>(SOCKET_GROUP, 1);
const SIOCSLOWAT: IoCmd = IoCmd::iow::<i32>(SOCKET_GROUP, 2);
const SIOCGLOWAT: IoCmd = IoCmd::ior::<i32>(SOCKET_GROUP, 3);
const SIOCATMARK: IoCmd = IoCmd::ior::<i32>(SOCKET_GROUP, 7);
const SIOCSPGRP: IoCmd = IoCmd::iow::<i32>(SOCKET_GROUP, 8);
const SIOCGPGRP: IoCmd = IoCmd::ior::<i32>(SOCKET_GROUP, 9);
