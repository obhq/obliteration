use self::cache::UmaCache;
use crate::config::config;
use crate::context::Context;
use alloc::borrow::Cow;
use alloc::vec::Vec;

mod bucket;
mod cache;

/// Implementation of `uma_zone` structure.
pub struct UmaZone {
    size: usize,           // uz_size
    caches: Vec<UmaCache>, // uz_cpu
}

impl UmaZone {
    /// See `uma_zcreate` on the PS4 for a reference.
    pub fn new(_: Cow<'static, str>, size: usize, _: usize) -> Self {
        // Ths PS4 allocate a new uma_zone from masterzone_z but we don't have that. This method
        // basically an implementation of zone_ctor.
        let len = config().max_cpu.get();
        let mut caches = Vec::with_capacity(len);

        for _ in 0..len {
            caches.push(UmaCache::default());
        }

        Self {
            size, // TODO: Check if size is allowed to be zero. If not, change it to NonZero<usize>.
            caches,
        }
    }

    pub fn size(&self) -> usize {
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
        let cx = Context::pin();
        let cache = &self.caches[cx.cpu()];
        let bucket = cache.alloc();

        while let Some(bucket) = bucket {
            if bucket.len() != 0 {
                todo!()
            }

            todo!()
        }

        todo!()
    }
}
