pub use self::zone::*;

use crate::config::PAGE_SIZE;
use alloc::string::String;
use alloc::sync::Arc;
use core::num::NonZero;
use core::sync::atomic::AtomicBool;
use macros::bitflag;

mod bucket;
mod keg;
mod slab;
mod zone;

/// Implementation of UMA system.
pub struct Uma {
    bucket_enable: AtomicBool,
}

impl Uma {
    /// `UMA_SMALLEST_UNIT`.
    const SMALLEST_UNIT: NonZero<usize> = NonZero::new(PAGE_SIZE.get() / 256).unwrap();

    /// `UMA_MAX_WASTE`.
    const MAX_WASTE: NonZero<usize> = NonZero::new(PAGE_SIZE.get() / 10).unwrap();

    /// See `uma_startup` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13CA70|
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            bucket_enable: AtomicBool::new(true), // TODO: Use a proper value.
        })
    }

    /// See `uma_zcreate` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13DC80|
    pub fn create_zone(
        self: Arc<Self>,
        name: impl Into<String>,
        size: NonZero<usize>,
        align: Option<usize>,
        flags: UmaFlags,
    ) -> UmaZone {
        // The Orbis will allocate a new zone from masterzone_z. We choose to remove this since it
        // does not idomatic to Rust, which mean our uma_zone itself can live on the stack.
        UmaZone::new(self, name, None, size, align, flags)
    }
}

/// Flags for [`Uma::create_zone()`].
#[bitflag(u32)]
pub enum UmaFlags {
    /// `UMA_ZONE_ZINIT`.
    ZInit = 0x2,
    /// `UMA_ZONE_OFFPAGE`.
    Offpage = 0x8,
    /// `UMA_ZONE_MALLOC`.
    Malloc = 0x10,
    /// `UMA_ZONE_MTXCLASS`.
    MtxClass = 0x40,
    /// `UMA_ZONE_VM`.
    Vm = 0x80,
    /// `UMA_ZONE_HASH`.
    Hash = 0x100,
    /// `UMA_ZONE_SECONDARY`.
    Secondary = 0x200,
    /// `UMA_ZONE_REFCNT`.
    RefCnt = 0x400,
    /// `UMA_ZONE_MAXBUCKET`.
    MaxBucket = 0x800,
    /// `UMA_ZONE_CACHESPREAD`.
    CacheSpread = 0x1000,
    /// `UMA_ZONE_VTOSLAB`.
    VToSlab = 0x2000,
    /// `UMA_ZFLAG_INTERNAL`.
    Internal = 0x20000000,
    /// `UMA_ZFLAG_CACHEONLY`.
    CacheOnly = 0x80000000,
}
