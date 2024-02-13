use bitflags::bitflags;
use std::{ops::Deref, sync::Arc};

#[derive(Debug, Clone)]
pub struct Prison {
    parent: Option<StaticOrArc<Self>>,
    flags: PrisonFlags,
    allow: PrisonAllow,
}

impl Prison {
    pub fn flags(&self) -> PrisonFlags {
        self.flags
    }

    pub fn allow(&self) -> PrisonAllow {
        self.allow
    }
}

// TODO: move this somewhere else
#[derive(Debug, Clone)]
pub enum StaticOrArc<T: 'static> {
    Static(&'static T),
    Arc(Arc<T>),
}

impl<T> PartialEq<StaticOrArc<T>> for StaticOrArc<T> {
    fn eq(&self, other: &StaticOrArc<T>) -> bool {
        match (self, other) {
            (Self::Static(p1), Self::Static(p2)) => *p1 as *const T == *p2 as *const T,
            (Self::Arc(p1), Self::Arc(p2)) => Arc::ptr_eq(p1, p2),
            _ => false,
        }
    }
}

impl Eq for StaticOrArc<Prison> {}

impl<T> Deref for StaticOrArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Self::Static(p) => p,
            Self::Arc(p) => p,
        }
    }
}

impl<T> AsRef<T> for StaticOrArc<T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Static(p) => p,
            Self::Arc(p) => p,
        }
    }
}

pub type PrisonImpl = StaticOrArc<Prison>;

impl PrisonImpl {
    pub fn is_child(&self, other: &Self) -> bool {
        let mut p = other.parent.as_ref();

        while let Some(pr) = p {
            if self == pr {
                return true;
            }

            p = pr.parent.as_ref();
        }

        false
    }
}

pub static PRISON0: Prison = Prison {
    parent: None,
    flags: PrisonFlags::DEFAULT,
    allow: PrisonAllow::ALLOW_ALL,
};

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PrisonFlags: u32 {
        const HOST = 0x00000002;
        const VNET = 0x00000010;
        const IP4_SADDRSEL = 0x00000080;
        const IP6 = 0x04000000;

        const DEFAULT = Self::HOST.bits() | Self::VNET.bits() | Self::IP4_SADDRSEL.bits();
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PrisonAllow: u32 {
        const ALLOW_SOCKET_AF = 0x00000040;
        const ALLOW_ALL = 0x07ff;
    }
}
