use super::UmaKeg;
use crate::lock::Mutex;
use crate::mem::{RefCnt, too_many_refs};
use core::marker::PhantomPinned;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use macros::bitflag;

/// Implementation of `uma_slab`.
///
/// Unlike Orbis, we don't support `uma_slab_refcnt`. We use [alloc::sync::Arc] for that job instead
/// so you need to wrap the item to allocate from the slab with [alloc::sync::Arc] for any zones
/// that going to create with `UMA_ZONE_REFCNT`.
///
/// We use slightly different mechanism here but has the same memory layout (except we don't support
/// `uma_slab_refcnt` as stated on the above).
///
/// # Safety
/// Adding more fields into this struct without knowing how it work can cause undefined behavior in
/// some places.
#[repr(C)]
pub struct Slab {
    pub(super) pin: PhantomPinned,
    pub(super) hdr: SlabHdr, // us_head
    pub(super) free: [u8],   // us_freelist
}

impl Slab {
    pub fn flags(&self) -> SlabFlags {
        self.hdr.flags
    }

    /// Unlike Orbis, this method will return null if the slab already full instead of trigger a UB.
    ///
    /// See `slab_alloc_item` on the Orbis for a reference.
    ///
    /// # Safety
    /// This slab must be allocated from `k`.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x141FE0|
    pub unsafe fn alloc_item(&self, k: &mut UmaKeg) -> *mut u8 {
        // Check if full.
        let mut s = self.hdr.state.lock();

        if s.free_count == 0 {
            return null_mut();
        }

        // Allocate.
        let f = usize::from(s.first_free);

        s.first_free = self.free[f];
        s.free_count -= 1;
        k.free -= 1;

        if s.free_count == 0 {
            todo!()
        }

        self.hdr.refs.fetch_add(1, Ordering::Relaxed);

        unsafe { self.hdr.items.add(f * k.allocated_size()) }
    }
}

impl Drop for Slab {
    #[inline(never)]
    fn drop(&mut self) {
        core::sync::atomic::fence(Ordering::Acquire);

        todo!()
    }
}

unsafe impl RefCnt for Slab {
    fn increase_ref(&self) {
        let p = self.hdr.refs.fetch_add(1, Ordering::Relaxed);

        if p == usize::MAX {
            too_many_refs();
        }
    }

    fn decrease_ref(&self) -> usize {
        self.hdr.refs.fetch_sub(1, Ordering::Release)
    }
}

/// Implementation of `uma_slab_head`.
pub(super) struct SlabHdr {
    items: *mut u8,   // us_data
    flags: SlabFlags, // us_flags
    state: Mutex<SlabState>,
    refs: AtomicUsize,
}

impl SlabHdr {
    /// # Safety
    /// - `items` cannot be null.
    /// - `len` must be a number of elements of the array at `items`.
    pub unsafe fn new(flags: SlabFlags, items: *mut u8, len: usize) -> Self {
        Self {
            items,
            flags,
            state: Mutex::new(SlabState {
                free_count: len,
                first_free: 0,
            }),
            refs: AtomicUsize::new(0),
        }
    }
}

/// Flags for [SlabHdr].
#[bitflag(u8)]
pub enum SlabFlags {
    /// `UMA_SLAB_PRIV`.
    Private = 0x08,
    /// `UMA_SLAB_MALLOC`.
    Malloc = 0x20,
}

/// Contains mutable data for [SlabHdr].
pub(super) struct SlabState {
    pub(super) free_count: usize, // us_freecount
    pub(super) first_free: u8,    // us_firstfree
}
