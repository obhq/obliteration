use macros::bitflag;

/// Implementation of `vm_page` structure.
pub struct VmPage {
    addr: u64,        // phys_addr
    order: usize,     // order
    flags: PageFlags, // flags
    segment: usize,   // segind
    unk1: u8,
}

impl VmPage {
    pub const FREE_ORDER: usize = 13; // VM_NFREEORDER

    pub fn new(addr: u64, segment: usize) -> Self {
        Self {
            addr,
            order: Self::FREE_ORDER,
            flags: PageFlags::zeroed(),
            segment,
            unk1: 0,
        }
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    pub fn order(&self) -> usize {
        self.order
    }

    pub fn flags(&self) -> PageFlags {
        self.flags
    }

    pub fn segment(&self) -> usize {
        self.segment
    }

    pub fn unk1(&self) -> u8 {
        self.unk1
    }
}

/// Flags of [VmPage].
#[bitflag(u8)]
pub enum PageFlags {
    /// `PG_CACHED`.
    Cached = 0x01,
}
