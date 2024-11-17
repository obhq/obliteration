pub use self::zone::*;
use alloc::borrow::Cow;
use core::num::NonZero;

mod bucket;
mod zone;

/// Implementation of UMA system.
pub struct Uma {}

impl Uma {
    /// See `uma_startup` on the PS4 for a reference. Beware that our implementation cannot access
    /// the CPU context due to this function can be called before context activation.
    ///
    /// # Context safety
    /// This function does not require a CPU context.
    pub fn new() -> Self {
        Self {}
    }

    /// See `uma_zcreate` on the PS4 for a reference.
    ///
    /// # Context safety
    /// This function does not require a CPU context on **stage 1** heap.
    pub fn create_zone(&mut self, _: Cow<'static, str>, _: NonZero<usize>, _: usize) -> UmaZone {
        todo!()
    }
}
