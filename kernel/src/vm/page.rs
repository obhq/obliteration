use crate::lock::Mutex;
use core::hash::{Hash, Hasher};
use macros::bitflag;

/// Implementation of `vm_page` structure.
pub struct VmPage {
    pub index: usize,
    pub vm: usize,
    pub addr: u64,      // phys_addr
    pub segment: usize, // segind
    /// This **MUST** be locked after free queue.
    pub state: Mutex<PageState>,
    pub unk1: u8,
}

impl VmPage {
    pub const FREE_ORDER: usize = 13; // VM_NFREEORDER

    pub fn new(index: usize, vm: usize, pool: usize, addr: u64, segment: usize) -> Self {
        Self {
            index,
            vm,
            addr,
            segment,
            state: Mutex::new(PageState {
                pool,
                order: Self::FREE_ORDER,
                flags: PageFlags::zeroed(),
                extended_flags: PageExtFlags::zeroed(),
                access: PageAccess::zeroed(),
                wire_count: 0,
                act_count: 0,
                pindex: 0,
            }),
            unk1: 0,
        }
    }
}

impl PartialEq for VmPage {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl Eq for VmPage {}

impl Hash for VmPage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
    }
}

/// Contains mutable data for [VmPage];
pub struct PageState {
    pub pool: usize,                  // pool
    pub order: usize,                 // order
    pub flags: PageFlags,             // flags
    pub extended_flags: PageExtFlags, // oflags
    pub access: PageAccess,           // aflags
    pub wire_count: usize,            // wire_count
    pub act_count: u8,                // act_count
    pub pindex: usize,                // pindex
}

/// Value for [VmPage::flags].
#[bitflag(u8)]
pub enum PageFlags {
    /// `PG_CACHED`.
    Cached = 0x01,
    /// `PG_ZERO`.
    Zero = 0x08,
}

/// Value for [VmPage::extended_flags].
#[bitflag(u16)]
pub enum PageExtFlags {
    /// `VPO_BUSY`.
    Busy = 0x0001,
    /// `VPO_UNMANAGED`.
    Unmanaged = 0x0004,
}

/// Value for [VmPage::access].
#[bitflag(u8)]
pub enum PageAccess {}
