use super::bucket::UmaBucket;
use super::keg::UmaKeg;
use super::UmaFlags;
use crate::context::{current_thread, CpuLocal};
use crate::lock::{Gutex, GutexGroup};
use alloc::collections::VecDeque;
use alloc::string::String;
use core::cell::RefCell;
use core::cmp::min;
use core::num::NonZero;
use core::ops::DerefMut;
use core::ptr::null_mut;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    ty: ZoneType,
    size: NonZero<usize>,                     // uz_size
    caches: CpuLocal<RefCell<UmaCache>>,      // uz_cpu
    full_buckets: Gutex<VecDeque<UmaBucket>>, // uz_full_bucket
    free_buckets: Gutex<VecDeque<UmaBucket>>, // uz_free_bucket
    alloc_count: Gutex<u64>,                  // uz_allocs
    free_count: Gutex<u64>,                   // uz_frees
    count: Gutex<usize>,                      // uz_count
}

impl UmaZone {
    const ALIGN_CACHE: usize = 63; // uma_align_cache
    const BUCKET_MAX: usize = 128;

    /// See `zone_ctor` on Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13D490|
    pub(super) fn new(
        name: impl Into<String>,
        keg: Option<UmaKeg>,
        size: NonZero<usize>,
        align: Option<usize>,
        flags: UmaFlags,
    ) -> Self {
        let name = name.into();
        let keg = if flags.has(UmaFlags::Secondary) {
            todo!()
        } else {
            // We use a different approach here to make it idiomatic to Rust. On Orbis it will
            // construct a keg here if it is passed from the caller. If not it will allocate a new
            // keg from masterzone_k.
            keg.unwrap_or_else(|| UmaKeg::new(size, align.unwrap_or(Self::ALIGN_CACHE), flags))
        };

        // Get type and uz_count.
        let mut ty = ZoneType::Other;
        let mut count = 0;

        if !keg.flags().has(UmaFlags::Internal) {
            count = if !keg.flags().has(UmaFlags::MaxBucket) {
                min(keg.item_per_slab(), Self::BUCKET_MAX)
            } else {
                Self::BUCKET_MAX
            };

            match name.as_str() {
                "mbuf_packet" => {
                    ty = ZoneType::MBufPacket;
                    count = 4;
                }
                "mbuf_cluster_pack" => {
                    ty = ZoneType::MBufClusterPack;
                    count = Self::BUCKET_MAX;
                }
                "mbuf_jumbo_page" => {
                    ty = ZoneType::MBufJumboPage;
                    count = 1;
                }
                "mbuf" => {
                    ty = ZoneType::MBuf;
                    count = 16;
                }
                "mbuf_cluster" => {
                    ty = ZoneType::MBufCluster;
                    count = 1;
                }
                _ => (),
            }
        }

        // Construct uma_zone.
        let gg = GutexGroup::new();

        Self {
            ty,
            size: keg.size(),
            caches: CpuLocal::new(|_| RefCell::default()),
            full_buckets: gg.clone().spawn_default(),
            free_buckets: gg.clone().spawn_default(),
            alloc_count: gg.clone().spawn_default(),
            free_count: gg.clone().spawn_default(),
            count: gg.spawn(count),
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
    pub fn alloc(&self) -> *mut u8 {
        // Our implementation imply M_WAITOK and M_ZERO.
        let td = current_thread();

        if !td.can_sleep() {
            panic!("heap allocation in a non-sleeping context is not supported");
        }

        // Try allocate from per-CPU cache first so we don't need to acquire a mutex lock.
        let caches = self.caches.lock();
        let mem = Self::alloc_from_cache(caches.borrow_mut().deref_mut());

        if !mem.is_null() {
            return mem;
        }

        drop(caches); // Exit from non-sleeping context before acquire the mutex.

        // Cache not found, allocate from the zone. We need to re-check the cache again because we
        // may on a different CPU since we drop the CPU pinning on the above.
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
            ZoneType::MBufPacket | ZoneType::MBufJumboPage | ZoneType::MBuf | ZoneType::MBufCluster
        ) {
            todo!()
        }

        // TODO: What is this?
        if !matches!(
            self.ty,
            ZoneType::MBufCluster
                | ZoneType::MBuf
                | ZoneType::MBufJumboPage
                | ZoneType::MBufPacket
                | ZoneType::MBufClusterPack
        ) && *count < Self::BUCKET_MAX
        {
            *count += 1;
        }

        self.alloc_bucket();

        todo!()
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
    fn alloc_bucket(&self) -> bool {
        todo!()
    }
}

/// Type of [`UmaZone`].
#[derive(Clone, Copy)]
enum ZoneType {
    Other,
    /// `zone_pack`.
    MBufPacket,
    /// `zone_jumbop`.
    MBufJumboPage,
    /// `zone_mbuf`.
    MBuf,
    /// `zone_clust`.
    MBufCluster,
    /// `zone_clust_pack`.
    MBufClusterPack,
}

/// Implementation of `uma_cache` structure.
#[derive(Default)]
struct UmaCache {
    alloc: Option<UmaBucket>, // uc_allocbucket
    free: Option<UmaBucket>,  // uc_freebucket
    allocs: u64,              // uc_allocs
    frees: u64,               // uc_frees
}
