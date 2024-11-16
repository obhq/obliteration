use self::bucket::UmaBucket;
use crate::context::{current_thread, CpuLocal};
use crate::lock::{Gutex, GutexGroup};
use alloc::borrow::Cow;
use core::cell::RefCell;
use core::num::NonZero;
use core::ops::DerefMut;
use core::ptr::null_mut;

mod bucket;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    size: NonZero<usize>,                // uz_size
    caches: CpuLocal<RefCell<UmaCache>>, // uz_cpu
    allocs: Gutex<u64>,                  // uz_allocs
    frees: Gutex<u64>,                   // uz_frees
}

impl UmaZone {
    /// See `uma_zcreate` on the PS4 for a reference.
    ///
    /// # Context safety
    /// This function does not require a CPU context on **stage 1** heap.
    pub fn new(_: Cow<'static, str>, size: NonZero<usize>, _: usize) -> Self {
        // Ths PS4 allocate a new uma_zone from masterzone_z but we don't have that. This method
        // basically an implementation of zone_ctor.
        let gg = GutexGroup::new();

        Self {
            size,
            caches: CpuLocal::new(|_| RefCell::default()),
            allocs: gg.clone().spawn(0),
            frees: gg.spawn(0),
        }
    }

    pub fn size(&self) -> NonZero<usize> {
        self.size
    }

    /// See `uma_zalloc_arg` on the PS4 for a reference.
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
        let mut allocs = self.allocs.write();
        let mut frees = self.frees.write();
        let caches = self.caches.lock();
        let mut cache = caches.borrow_mut();
        let mem = Self::alloc_from_cache(&mut cache);

        if !mem.is_null() {
            return mem;
        }

        *allocs += core::mem::take(&mut cache.allocs);
        *frees += core::mem::take(&mut cache.frees);

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
}

/// Implementation of `uma_cache` structure.
#[derive(Default)]
struct UmaCache {
    alloc: Option<UmaBucket>, // uc_allocbucket
    free: Option<UmaBucket>,  // uc_freebucket
    allocs: u64,              // uc_allocs
    frees: u64,               // uc_frees
}
