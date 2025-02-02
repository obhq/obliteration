use super::bucket::{BucketItem, UmaBucket};
use super::keg::UmaKeg;
use super::{Alloc, Uma, UmaBox, UmaFlags};
use crate::context::{current_thread, CpuLocal};
use crate::lock::{Gutex, GutexGroup, GutexWrite};
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::cmp::min;
use core::num::NonZero;
use core::ops::DerefMut;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    bucket_enable: Arc<AtomicBool>,
    bucket_keys: Arc<Vec<usize>>,
    bucket_zones: Arc<Vec<UmaZone>>,
    ty: ZoneType,
    size: NonZero<usize>,                                           // uz_size
    caches: CpuLocal<RefCell<UmaCache>>,                            // uz_cpu
    full_buckets: Gutex<VecDeque<UmaBox<UmaBucket<[BucketItem]>>>>, // uz_full_bucket
    free_buckets: Gutex<VecDeque<UmaBox<UmaBucket<[BucketItem]>>>>, // uz_free_bucket
    alloc_count: Gutex<u64>,                                        // uz_allocs
    free_count: Gutex<u64>,                                         // uz_frees
    count: Gutex<usize>,                                            // uz_count
    flags: UmaFlags,                                                // uz_flags
}

impl UmaZone {
    const ALIGN_CACHE: usize = 63; // uma_align_cache

    /// See `zone_ctor` on Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13D490|
    #[allow(clippy::too_many_arguments)] // TODO: Find a better way.
    pub(super) fn new(
        bucket_enable: Arc<AtomicBool>,
        bucket_keys: Arc<Vec<usize>>,
        bucket_zones: Arc<Vec<UmaZone>>,
        name: impl Into<String>,
        keg: Option<UmaKeg>,
        size: NonZero<usize>,
        align: Option<usize>,
        flags: UmaFlags,
    ) -> Self {
        let name = name.into();
        let (keg, flags) = if flags.has(UmaFlags::Secondary) {
            todo!()
        } else {
            // We use a different approach here to make it idiomatic to Rust. On Orbis it will
            // construct a keg here if it is passed from the caller. If not it will allocate a new
            // keg from masterzone_k.
            let keg = match keg {
                Some(v) => v,
                None => UmaKeg::new(size, align.unwrap_or(Self::ALIGN_CACHE), flags),
            };

            (keg, UmaFlags::zeroed())
        };

        // Get type and uz_count.
        let mut ty = ZoneType::Other;
        let mut count = 0;

        if !keg.flags().has(UmaFlags::Internal) {
            count = if !keg.flags().has(UmaFlags::MaxBucket) {
                min(keg.item_per_slab(), Uma::BUCKET_MAX)
            } else {
                Uma::BUCKET_MAX
            };

            match name.as_str() {
                "mbuf_packet" => {
                    ty = ZoneType::MbufPacket;
                    count = 4;
                }
                "mbuf_cluster_pack" => {
                    ty = ZoneType::MbufClusterPack;
                    count = Uma::BUCKET_MAX;
                }
                "mbuf_jumbo_page" => {
                    ty = ZoneType::MbufJumboPage;
                    count = 1;
                }
                "mbuf" => {
                    ty = ZoneType::Mbuf;
                    count = 16;
                }
                "mbuf_cluster" => {
                    ty = ZoneType::MbufCluster;
                    count = 1;
                }
                _ => (),
            }
        }

        // Construct uma_zone.
        let gg = GutexGroup::new();

        Self {
            bucket_enable,
            bucket_keys,
            bucket_zones,
            ty,
            size: keg.size(),
            caches: CpuLocal::new(|_| RefCell::default()),
            full_buckets: gg.clone().spawn_default(),
            free_buckets: gg.clone().spawn_default(),
            alloc_count: gg.clone().spawn_default(),
            free_count: gg.clone().spawn_default(),
            count: gg.spawn(count),
            flags: flags | (keg.flags() & UmaFlags::from(0xa2002518)), // TODO: Use named flags.
        }
    }

    pub fn size(&self) -> NonZero<usize> {
        self.size
    }

    /// See `uma_zalloc_arg` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13E750|
    pub fn alloc(&self, flags: Alloc) -> *mut u8 {
        if flags.has(Alloc::Wait) {
            // TODO: The Orbis also modify td_pflags on a certain condition.
            let td = current_thread();

            if !td.can_sleep() {
                panic!("attempt to do waitable heap allocation in a non-sleeping context");
            }
        }

        loop {
            // Try allocate from per-CPU cache first so we don't need to acquire a mutex lock.
            let caches = self.caches.lock();
            let mem = Self::alloc_from_cache(caches.borrow_mut().deref_mut());

            if !mem.is_null() {
                return mem;
            }

            drop(caches); // Exit from non-sleeping context before acquire the mutex.

            // Cache not found, allocate from the zone. We need to re-check the cache again because
            // we may on a different CPU since we drop the CPU pinning on the above.
            let mut frees = self.free_buckets.write();
            let mut count = self.count.write();
            let caches = self.caches.lock();
            let mut cache = caches.borrow_mut();
            let mem = Self::alloc_from_cache(&mut cache);

            if !mem.is_null() {
                return mem;
            }

            // TODO: What actually we are doing here?
            *self.alloc_count.write() += core::mem::take(&mut cache.allocs);
            *self.free_count.write() += core::mem::take(&mut cache.frees);

            if let Some(b) = cache.alloc.take() {
                frees.push_front(b);
            }

            if let Some(b) = self.full_buckets.write().pop_front() {
                cache.alloc = Some(b);

                // Seems like this should never fail.
                let m = Self::alloc_from_cache(&mut cache);

                assert!(!m.is_null());

                return m;
            }

            drop(cache);
            drop(caches);

            // TODO: What is this?
            if matches!(
                self.ty,
                ZoneType::MbufPacket
                    | ZoneType::MbufJumboPage
                    | ZoneType::Mbuf
                    | ZoneType::MbufCluster
            ) {
                if flags.has(Alloc::Wait) {
                    todo!()
                }

                todo!()
            }

            // TODO: What is this?
            if !matches!(
                self.ty,
                ZoneType::MbufCluster
                    | ZoneType::Mbuf
                    | ZoneType::MbufJumboPage
                    | ZoneType::MbufPacket
                    | ZoneType::MbufClusterPack
            ) && *count < Uma::BUCKET_MAX
            {
                *count += 1;
            }

            if self.alloc_bucket(frees, count, flags) {
                return self.alloc_item(flags);
            }
        }
    }

    fn alloc_from_cache(c: &mut UmaCache) -> *mut u8 {
        while let Some(b) = &mut c.alloc {
            if b.len() != 0 {
                todo!()
            }

            if c.free.as_ref().is_some_and(|b| b.len() != 0) {
                core::mem::swap(&mut c.alloc, &mut c.free);
                continue;
            }

            break;
        }

        null_mut()
    }

    /// See `zone_alloc_bucket` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13EBA0|
    fn alloc_bucket(
        &self,
        frees: GutexWrite<VecDeque<UmaBox<UmaBucket<[BucketItem]>>>>,
        count: GutexWrite<usize>,
        flags: Alloc,
    ) -> bool {
        match frees.front() {
            Some(_) => todo!(),
            None => {
                if self.bucket_enable.load(Ordering::Relaxed) {
                    // Get allocation flags.
                    let mut flags = flags & !Alloc::Zero;

                    if self.flags.has(UmaFlags::CacheOnly) {
                        flags |= Alloc::NoVm;
                    }

                    // Alloc a bucket.
                    let i = (*count + 15) >> Uma::BUCKET_SHIFT;
                    let k = self.bucket_keys[i];

                    self.bucket_zones[k].alloc_item(flags);

                    todo!()
                }
            }
        }

        true
    }

    /// See `zone_alloc_item` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13DD50|
    fn alloc_item(&self, _: Alloc) -> *mut u8 {
        todo!()
    }
}

/// Type of [`UmaZone`].
#[derive(Clone, Copy)]
enum ZoneType {
    Other,
    /// `zone_pack`.
    MbufPacket,
    /// `zone_jumbop`.
    MbufJumboPage,
    /// `zone_mbuf`.
    Mbuf,
    /// `zone_clust`.
    MbufCluster,
    /// `zone_clust_pack`.
    MbufClusterPack,
}

/// Implementation of `uma_cache` structure.
#[derive(Default)]
struct UmaCache {
    alloc: Option<UmaBox<UmaBucket<[BucketItem]>>>, // uc_allocbucket
    free: Option<UmaBox<UmaBucket<[BucketItem]>>>,  // uc_freebucket
    allocs: u64,                                    // uc_allocs
    frees: u64,                                     // uc_frees
}
