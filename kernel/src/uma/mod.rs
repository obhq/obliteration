pub use self::zone::*;

use alloc::string::String;
use alloc::sync::Arc;
use bitfield_struct::bitfield;
use core::num::NonZero;

mod bucket;
mod keg;
mod slab;
mod zone;

/// Implementation of UMA system.
pub struct Uma {}

impl Uma {
    /// See `uma_startup` on the Orbis for a reference. Beware that our implementation cannot access
    /// the CPU context due to this function can be called before context activation.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13CA70|
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    /// See `uma_zcreate` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13DC80|
    pub fn create_zone(
        &self,
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
    __: bool,
    pub zinit: bool,
    #[bits(2)]
    __: u8,
    /// `UMA_ZONE_MALLOC`.
    pub malloc: bool,
    #[bits(2)]
    __: u8,
    /// `UMA_ZONE_VM`.
    pub vm: bool,
    __: bool,
    /// `UMA_ZONE_SECONDARY`.
    pub secondary: bool,
    /// `UMA_ZONE_REFCNT`.
    pub refcnt: bool,
    __: bool,
    /// `UMA_ZONE_CACHESPREAD`.
    pub cache_spread: bool,
    /// `UMA_ZONE_VTOSLAB`.
    pub vtoslab: bool,
    #[bits(15)]
    __: u32,
    /// `UMA_ZFLAG_INTERNAL`.
    pub internal: bool,
    __: bool,
    __: bool,
}
