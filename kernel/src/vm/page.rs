use macros::bitflag;

/// Implementation of `vm_page` structure.
pub struct VmPage {
    flags: PageFlags, // flags
}

impl VmPage {
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
