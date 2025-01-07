pub use self::zone::*;
use alloc::borrow::Cow;
use bitfield_struct::bitfield;
use core::num::NonZero;

mod bucket;
mod zone;

/// Implementation of UMA system.
pub struct Uma {}

impl Uma {
    /// See `uma_startup` on the Orbis for a reference. Beware that our implementation cannot access
    /// the CPU context due to this function can be called before context activation.
    ///
    /// # Context safety
    /// This function does not require a CPU context.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13CA70|
    pub fn new() -> Self {
        Self {}
    }

    /// See `uma_zcreate` on the Orbis for a reference.
    ///
    /// # Context safety
    /// This function does not require a CPU context on **stage 1** heap.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13DC80|
    pub fn create_zone(
        &mut self,
        _: Cow<'static, str>,
        _: NonZero<usize>,
        _: usize,
        _: UmaFlags,
    ) -> UmaZone {
        // The Orbis will allocate a new zone from masterzone_z. We choose to remove this since it
        // does not idomatic to Rust, which mean our uma_zone itself can live on the stack.
        UmaZone::new()
    }
}

/// Flags for [`Uma::create_zone()`].
#[bitfield(u32)]
pub struct UmaFlags {
    #[bits(4)]
    __: u8,
    /// `UMA_ZONE_MALLOC`.
    pub malloc: bool,
    #[bits(27)]
    __: u32,
}
