/// Implementation of `uma_slab_head`, `uma_slab` and `uma_slab_refcnt`.
///
/// We use slightly different mechanism here but has the same memory layout.
#[repr(C)]
pub struct Slab<I: ?Sized> {
    pub free: I, // us_freelist
}

impl<I: ?Sized> Slab<I> {
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

/// Item in the slab to represents `uma_slab` structure.
#[repr(C)]
pub struct Free {
    pub item: u8, // us_item
}

/// Item in the slab to represents `uma_slab_refcnt` structure.
#[repr(C)]
pub struct RcFree {
    pub item: u8,    // us_item
    pub refcnt: u32, // us_refcnt
}
