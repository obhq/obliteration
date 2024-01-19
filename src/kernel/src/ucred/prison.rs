use bitflags::bitflags;

#[derive(Debug)]
pub struct Prison {
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

pub static PRISON0: Prison = Prison {
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

        //SelF::HOST | Self::VNET | Self::IP4_SADDRSEL wouldn't be allowed in a const context
        const DEFAULT = 0x00000002 | 0x00000010 | 0x00000080;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PrisonAllow: u32 {
        const ALLOW_SOCKET_AF = 0x00000040;
        const ALLOW_ALL = 0x07ff;

    }
}
