pub use self::boxed::*;
pub use self::zone::*;

use self::bucket::{BucketItem, UmaBucket};
use crate::config::PAGE_SIZE;
use crate::vm::Vm;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::num::NonZero;
use core::sync::atomic::AtomicBool;
use macros::bitflag;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod boxed;
mod bucket;
mod keg;
mod slab;
mod zone;

/// Implementation of UMA system.
pub struct Uma {
    vm: Arc<Vm>,
    bucket_enable: Arc<AtomicBool>,
    bucket_keys: Arc<Vec<usize>>,    // bucket_size
    bucket_zones: Arc<Vec<UmaZone>>, // bucket_zones
}

impl Uma {
    /// `UMA_SMALLEST_UNIT`.
    const SMALLEST_UNIT: NonZero<usize> = NonZero::new(PAGE_SIZE.get() / 256).unwrap();

    /// `UMA_MAX_WASTE`.
    const MAX_WASTE: NonZero<usize> = NonZero::new(PAGE_SIZE.get() / 10).unwrap();
    const BUCKET_MAX: usize = 128;
    const BUCKET_SHIFT: usize = 4;

    /// `bucket_zones`.
    const BUCKET_SIZES: [usize; 4] = [16, 32, 64, 128];

    /// See `uma_startup` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13CA70|
    pub fn new(vm: Arc<Vm>) -> Arc<Self> {
        let bucket_enable = Arc::new(AtomicBool::new(true)); // TODO: Use a proper value.
        let mut bucket_keys = Vec::new();
        let mut bucket_zones = Vec::with_capacity(Self::BUCKET_SIZES.len());
        let mut ki = 0;

        // Create bucket zones.
        for (si, size) in Self::BUCKET_SIZES.into_iter().enumerate() {
            let items = Layout::array::<BucketItem>(size).unwrap();
            let layout = Layout::new::<UmaBucket<()>>()
                .extend(items)
                .unwrap()
                .0
                .pad_to_align();

            bucket_zones.push(UmaZone::new(
                vm.clone(),
                bucket_enable.clone(),
                Arc::default(),
                Arc::default(),
                format!("{size} Bucket"),
                None,
                layout.size().try_into().unwrap(),
                Some(layout.align() - 1),
                UmaFlags::Bucket | UmaFlags::Internal,
            ));

            while ki <= size {
                bucket_keys.push(si);
                ki += 1 << Self::BUCKET_SHIFT;
            }
        }

        Arc::new(Self {
            vm,
            bucket_enable,
            bucket_keys: Arc::new(bucket_keys),
            bucket_zones: Arc::new(bucket_zones),
        })
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
        UmaZone::new(
            self.vm.clone(),
            self.bucket_enable.clone(),
            self.bucket_keys.clone(),
            self.bucket_zones.clone(),
            name,
            None,
            size,
            align,
            flags,
        )
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
    /// `UMA_ZFLAG_BUCKET`.
    Bucket = 0x2000000,
    /// `UMA_ZFLAG_INTERNAL`.
    Internal = 0x20000000,
    /// `UMA_ZFLAG_CACHEONLY`.
    CacheOnly = 0x80000000,
}

/// Implementation of `malloc` flags.
#[bitflag(u32)]
pub enum Alloc {
    /// `M_NOWAIT`.
    NoWait = 0x1,
    /// `M_WAITOK`.
    Wait = 0x2,
    /// `M_ZERO`.
    Zero = 0x100,
    /// `M_NOVM`.
    NoVm = 0x200,
}
