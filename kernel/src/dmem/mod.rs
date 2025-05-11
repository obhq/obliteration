use crate::MemoryInfo;
use alloc::sync::Arc;

/// Implementation of Direct Memory system.
pub struct Dmem {}

impl Dmem {
    /// See `initialize_dmem` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F5C20|
    pub fn new(_: &mut MemoryInfo) -> Arc<Self> {
        todo!()
    }
}
