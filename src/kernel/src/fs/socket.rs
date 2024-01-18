use std::{num::NonZeroI32, sync::Arc};

use crate::{
    errno::{Errno, EPERM, EPROTONOSUPPORT, EPROTOTYPE},
    process::VThread,
    ucred::{PrisonCheckAfError, Ucred},
};
use thiserror::Error;

#[derive(Debug)]
pub struct Socket {
    proto: &'static Protosw,
    ty: i32,
    cred: Arc<Ucred>,
}

impl Socket {
    /// See `socreate` on the PS4 for a reference.
    pub(super) fn new(
        domain: i32,
        ty: i32,
        proto: i32,
        cred: &Arc<Ucred>,
        td: &VThread,
    ) -> Result<Self, SocketCreateError> {
        //ipv6
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

        if prp.ty != ty {}

        //TODO

        cred.prison_check_address_family(0)?;

        Ok(Self {
            proto: prp,
            ty,
            cred: cred.clone(),
        })

        todo!()
    }
}

#[derive(Debug)]
pub(super) struct Protosw {
    ty: i32,
}

impl Protosw {
    pub(super) fn find_by_proto(domain: i32, protocol: i32, ty: i32) -> Option<&'static Self> {
        todo!()
    }

    pub(super) fn find_by_type(domain: i32, ty: i32) -> Option<&'static Self> {
        todo!()
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
