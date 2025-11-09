use crate::lock::Mutex;
use core::hash::{Hash, Hasher};
use macros::bitflag;

/// Implementation of `vm_page` structure.
pub struct VmPage {
    vm: usize,
    pool: usize,         // pool
    addr: u64,           // phys_addr
    order: Mutex<usize>, // order
    flags: PageFlags,    // flags
    segment: usize,      // segind
    unk1: u8,
}

impl VmPage {
    pub const FREE_ORDER: usize = 13; // VM_NFREEORDER

    pub fn new(vm: usize, pool: usize, addr: u64, segment: usize) -> Self {
        Self {
            vm,
            pool,
            addr,
            order: Mutex::new(Self::FREE_ORDER),
            flags: PageFlags::zeroed(),
            segment,
            unk1: 0,
        }
    }

    pub fn vm(&self) -> usize {
        self.vm
    }

    pub fn pool(&self) -> usize {
        self.pool
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    /// This must be locked **after** free queues and the lock must be held while putting this page
    /// to free queues.
    pub fn order(&self) -> &Mutex<usize> {
        &self.order
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

/// Flags of [VmPage].
#[bitflag(u8)]
pub enum PageFlags {
    /// `PG_CACHED`.
    Cached = 0x01,
}
