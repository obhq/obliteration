use crate::lock::Mutex;
use core::hash::{Hash, Hasher};
use macros::bitflag;

/// Implementation of `vm_page` structure.
pub struct VmPage {
    index: usize,
    vm: usize,
    pool: Mutex<usize>,                  // pool
    addr: u64,                           // phys_addr
    order: Mutex<usize>,                 // order
    flags: Mutex<PageFlags>,             // flags
    extended_flags: Mutex<PageExtFlags>, // oflags
    access: Mutex<PageAccess>,           // aflags
    segment: usize,                      // segind
    unk1: u8,
}

impl VmPage {
    pub const FREE_ORDER: usize = 13; // VM_NFREEORDER

    pub fn new(index: usize, vm: usize, pool: usize, addr: u64, segment: usize) -> Self {
        Self {
            index,
            vm,
            pool: Mutex::new(pool),
            addr,
            order: Mutex::new(Self::FREE_ORDER),
            flags: Mutex::new(PageFlags::zeroed()),
            extended_flags: Mutex::new(PageExtFlags::zeroed()),
            access: Mutex::new(PageAccess::zeroed()),
            segment,
            unk1: 0,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn vm(&self) -> usize {
        self.vm
    }

    /// This must be locked **after** [Self::order()] and the lock must be held while putting this
    /// page to free queues.
    pub fn pool(&self) -> &Mutex<usize> {
        &self.pool
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    /// This must be locked **after** free queues and the lock must be held while putting this page
    /// to free queues.
    pub fn order(&self) -> &Mutex<usize> {
        &self.order
    }

    pub fn flags(&self) -> &Mutex<PageFlags> {
        &self.flags
    }

    pub fn extended_flags(&self) -> &Mutex<PageExtFlags> {
        &self.extended_flags
    }

    pub fn access(&self) -> &Mutex<PageAccess> {
        &self.access
    }

    pub fn segment(&self) -> usize {
        self.segment
    }

    pub fn unk1(&self) -> u8 {
        self.unk1
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
