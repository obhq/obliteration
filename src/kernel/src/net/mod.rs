#![allow(unused_variables)]

use core::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressFamily(i32);

impl AddressFamily {
    pub const UNSPEC: Self = Self(0);
    pub const LOCAL: Self = Self::UNIX;
    pub const UNIX: Self = Self(1);
    pub const INET: Self = Self(2);
    pub const ROUTE: Self = Self(17);
    pub const INET6: Self = Self(28);
}

impl Display for AddressFamily {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UNSPEC => write!(f, "UNSPEC"),
            Self::LOCAL => write!(f, "LOCAL"),
            Self::INET => write!(f, "INET"),
            Self::ROUTE => write!(f, "ROUTE"),
            Self::INET6 => write!(f, "INET6"),
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct Protosw {
    ty: i32,
    domain: &'static Domain,
}

impl Protosw {
    pub fn ty(&self) -> i32 {
        self.ty
    }

    pub fn domain(&self) -> &'static Domain {
        self.domain
    }

    pub(super) fn find_by_proto(domain: i32, protocol: i32, ty: i32) -> Option<&'static Self> {
        todo!()
    }

    pub(super) fn find_by_type(domain: i32, ty: i32) -> Option<&'static Self> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Domain {
    family: AddressFamily,
}

impl Domain {
    pub fn family(&self) -> AddressFamily {
        self.family
    }
}
