use bitflags::bitflags;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Prison {
    parent: Option<Cow<'static, Self>>,
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

    pub fn is_child(&self, other: &Self) -> bool {
        let mut p = other.parent.as_ref();

        while let Some(pr) = p {
            if self == other {
                return true;
            }

            p = pr.parent.as_ref();
        }

        false
    }
}

impl PartialEq for Prison {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ToOwned for Prison {
    type Owned = Box<Self>;

    fn to_owned(&self) -> Self::Owned {
        Box::new(Prison {
            parent: self.parent.as_ref().map(|p| p.to_owned()),
            flags: self.flags,
            allow: self.allow,
        })
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
