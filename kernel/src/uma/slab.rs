/// Implementation of `uma_slab_head`, `uma_slab` and `uma_slab_refcnt`.
///
/// We use slightly different mechanism here but has the same memory layout.
#[repr(C)]
pub struct Slab<I: ?Sized> {
    free: I, // us_freelist
}

/// Item in the slab to represents `uma_slab` structure.
#[repr(C)]
pub struct Free {}

/// Item in the slab to represents `uma_slab_refcnt` structure.
#[repr(C)]
pub struct RcFree {}
