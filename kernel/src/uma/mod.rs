pub use self::zone::*;

use alloc::string::String;
use bitfield_struct::bitfield;
use core::num::NonZero;

mod bucket;
mod keg;
mod zone;

/// Implementation of UMA system.
pub struct Uma {}

impl Uma {
    /// See `uma_startup` on the Orbis for a reference. Beware that our implementation cannot access
    /// the CPU context due to this function can be called before context activation.
    ///
    /// # Context safety
    /// This function does not require a CPU context on **stage 1** heap.
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
        name: impl Into<String>,
        size: NonZero<usize>,
        align: Option<usize>,
        flags: UmaFlags,
    ) -> UmaZone {
        // The Orbis will allocate a new zone from masterzone_z. We choose to remove this since it
        // does not idomatic to Rust, which mean our uma_zone itself can live on the stack.
        UmaZone::new(name, None, size, align, flags)
    }
}

/// Flags for [`Uma::create_zone()`].
#[bitfield(u32)]
pub struct UmaFlags {
    #[bits(4)]
    __: u8,
    /// `UMA_ZONE_MALLOC`.
    pub malloc: bool,
    #[bits(4)]
    __: u8,
    /// `UMA_ZONE_SECONDARY`.
    pub secondary: bool,
    #[bits(19)]
    __: u32,
    /// `UMA_ZFLAG_INTERNAL`.
    pub internal: bool,
    __: bool,
    __: bool,
}
