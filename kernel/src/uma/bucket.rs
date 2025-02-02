/// Implementation of `uma_bucket` structure.
#[repr(C)]
pub struct UmaBucket<I: ?Sized> {
    len: usize, // ub_cnt
    items: I,   // ub_bucket
}

impl<I: ?Sized> UmaBucket<I> {
    pub fn len(&self) -> usize {
        self.len
    }
}

/// Each item in the [`UmaBucket::items`].
pub struct BucketItem {}
