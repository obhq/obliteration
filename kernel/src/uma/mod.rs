use self::cache::UmaCache;
use crate::context::{current_thread, CpuLocal};
use alloc::borrow::Cow;
use core::cell::RefCell;
use core::num::NonZero;

mod bucket;
mod cache;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    size: NonZero<usize>,                // uz_size
    caches: CpuLocal<RefCell<UmaCache>>, // uz_cpu
}

impl UmaZone {
    /// See `uma_zcreate` on the PS4 for a reference.
    pub fn new(_: Cow<'static, str>, size: NonZero<usize>, _: usize) -> Self {
        // Ths PS4 allocate a new uma_zone from masterzone_z but we don't have that. This method
        // basically an implementation of zone_ctor.
        Self {
            size,
            caches: CpuLocal::new(|_| RefCell::default()),
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

        // Try to allocate from per-CPU cache.
        let pin = self.caches.lock();
        let mut cache = pin.borrow_mut();
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
