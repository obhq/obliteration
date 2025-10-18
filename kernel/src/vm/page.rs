use macros::bitflag;

/// Implementation of `vm_page` structure.
pub struct VmPage {
    flags: PageFlags, // flags
}

impl VmPage {
    pub fn new() -> Self {
        Self {
            flags: PageFlags::zeroed(),
        }
    }

    pub fn flags(&self) -> PageFlags {
        self.flags
    }
}

/// Flags of [`VmPage`].
#[bitflag(u8)]
pub enum PageFlags {
    /// `PG_CACHED`.
    Cached = 0x01,
}
