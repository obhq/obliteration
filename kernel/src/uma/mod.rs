use self::cache::UmaCache;
use crate::context::{Context, CpuLocal};
use crate::lock::Mutex;
use alloc::borrow::Cow;
use core::num::NonZero;

mod bucket;
mod cache;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    size: NonZero<usize>,              // uz_size
    caches: CpuLocal<Mutex<UmaCache>>, // uz_cpu
}

impl UmaZone {
    /// See `uma_zcreate` on the PS4 for a reference.
    pub fn new(_: Cow<'static, str>, size: NonZero<usize>, _: usize) -> Self {
        // Ths PS4 allocate a new uma_zone from masterzone_z but we don't have that. This method
        // basically an implementation of zone_ctor.
        Self {
            size,
            caches: CpuLocal::new(|_| Mutex::new(UmaCache::default())),
        }
    }

    pub fn size(&self) -> NonZero<usize> {
        self.size
    }

    /// See `uma_zalloc_arg` on the PS4 for a reference.
    pub fn alloc(&self) -> *mut u8 {
        // Our implementation imply M_WAITOK and M_ZERO.
        let td = Context::thread();

        if td.active_interrupts() != 0 {
            panic!("heap allocation in an interrupt handler is not supported");
        }

        // Try to allocate from per-CPU cache.
        let pin = self.caches.lock();
        let mut cache = pin.lock();
        let bucket = cache.alloc_mut();

        while let Some(bucket) = bucket {
            if bucket.len() != 0 {
                todo!()
            }

            todo!()
        }

        drop(cache);
        drop(pin);

        todo!()
    }
}
