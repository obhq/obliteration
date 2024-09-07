use alloc::borrow::Cow;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {}

impl UmaZone {
    /// See `uma_zcreate` on the PS4 for a reference.
    pub fn new(_: Cow<'static, str>, _: usize, _: usize) -> Self {
        Self {}
    }

    /// See `uma_zalloc_arg` on the PS4 for a reference.
    pub fn alloc(&self) -> *mut u8 {
        todo!()
    }
}
