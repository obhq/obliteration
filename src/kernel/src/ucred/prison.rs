use bitflags::bitflags;
use lazy_static::lazy_static;
use std::sync::Arc;

#[derive(Debug)]
pub struct Prison {
    parent: Option<Arc<Self>>,
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

    pub fn is_child(self: &Arc<Self>, other: &Arc<Self>) -> bool {
        let mut p = other.parent.as_ref();

        while let Some(pr) = p {
            if Arc::ptr_eq(pr, self) {
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

lazy_static! {
    pub static ref PRISON0: Arc<Prison> = Arc::new(Prison {
        parent: None,
        flags: PrisonFlags::DEFAULT,
        allow: PrisonAllow::ALLOW_ALL,
    });
}

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
