use alloc::borrow::Cow;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    size: usize, // uz_size
}

impl UmaZone {
    /// See `uma_zcreate` on the PS4 for a reference.
    pub fn new(_: Cow<'static, str>, size: usize, _: usize) -> Self {
        // TODO: Check if size is allowed to be zero. If not, change it to NonZero<usize>.
        Self { size }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// See `uma_zalloc_arg` on the PS4 for a reference.
    pub fn alloc(&self) -> *mut u8 {
        todo!()
    }
}
