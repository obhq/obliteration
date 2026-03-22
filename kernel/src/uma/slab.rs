use super::{UmaFlags, UmaKeg};
use alloc::sync::Arc;

/// Implementation of `uma_slab` and `uma_slab_refcnt`.
///
/// We use slightly different mechanism here but has the same memory layout.
///
/// # Safety
/// Adding more fields into this struct without knowing how it work can cause undefined behavior in
/// some places.
#[repr(C)]
pub struct Slab<I> {
    pub hdr: SlabHdr<I>, // us_head
    pub free: [I],       // us_freelist
}

impl<I> Slab<I> {
    /// See `slab_alloc_item` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x141FE0|
    pub fn alloc_item(&mut self) {
        todo!()
    }
}

/// Implementation of `uma_slab_head`.
pub struct SlabHdr<I> {
    pub keg: Arc<UmaKeg<I>>, // us_keg
}

/// Item in [Slab::free] to represents `uma_slab` structure.
#[repr(C)]
pub struct StdFree {
    pub item: u8, // us_item
}

unsafe impl FreeItem for StdFree {
    fn new(idx: usize) -> Self {
        Self {
            item: (idx + 1).try_into().unwrap(),
        }
    }

    fn flags() -> UmaFlags {
        UmaFlags::zeroed()
    }
}

/// Item in [Slab::free] to represents `uma_slab_refcnt` structure.
#[repr(C)]
#[allow(dead_code)] // TODO: Remove this.
pub struct RefFree {
    pub item: u8,    // us_item
    pub refcnt: u32, // us_refcnt
}

unsafe impl FreeItem for RefFree {
    fn new(idx: usize) -> Self {
        Self {
            item: (idx + 1).try_into().unwrap(),
            refcnt: 0,
        }
    }

    fn flags() -> UmaFlags {
        UmaFlags::VToSlab.into()
    }
}

/// Each item in [Slab::free].
///
/// # Safety
/// Wrong flags from [Self::flags()] can cause undefined behavior in some places.
pub unsafe trait FreeItem: Sized {
    fn new(idx: usize) -> Self;
    fn flags() -> UmaFlags;
}
