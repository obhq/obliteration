use crate::config::PAGE_SIZE;
use crate::context::{current_thread, current_uma, CpuLocal};
use crate::uma::{UmaFlags, UmaZone};
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::cell::RefCell;
use core::num::NonZero;

/// Stage 2 kernel heap.
///
/// This stage allocate a memory from a virtual memory management system. This struct is a merge of
/// `malloc_type` and `malloc_type_internal` structure.
pub struct Stage2 {
    zones: [Vec<Arc<UmaZone>>; (usize::BITS - 1) as usize], // kmemsize + kmemzones
    stats: CpuLocal<RefCell<Stats>>,                        // mti_stats
}

impl Stage2 {
    const KMEM_ZSHIFT: usize = 4;
    const KMEM_ZBASE: usize = 16;
    const KMEM_ZMASK: usize = Self::KMEM_ZBASE - 1;
    const KMEM_ZSIZE: usize = PAGE_SIZE.get() >> Self::KMEM_ZSHIFT;

    /// See `kmeminit` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1A4B80|
    pub fn new() -> Self {
        // The possible of maximum alignment that Layout allowed is a bit before the most
        // significant bit of isize (e.g. 0x4000000000000000 on 64 bit system). So we can use
        // "size_of::<usize>() * 8 - 1" to get the size of array for all possible alignment.
        let uma = current_uma().unwrap();
        let zones = core::array::from_fn(|align| {
            let mut zones = Vec::with_capacity(Self::KMEM_ZSIZE + 1);
            let mut last = 0;
            let align = align
                .try_into()
                .ok()
                .and_then(|align| 1usize.checked_shl(align))
                .unwrap();

            for i in Self::KMEM_ZSHIFT.. {
                // Stop if size larger than page size.
                let size = NonZero::new(1usize << i).unwrap();

                if size > PAGE_SIZE {
                    break;
                }

                // Create zone.
                let zone = Arc::new(uma.create_zone(
                    size.to_string(),
                    size,
                    Some(align - 1),
                    UmaFlags::Malloc,
                ));

                while last <= size.get() {
                    zones.push(zone.clone());
                    last += Self::KMEM_ZBASE;
                }
            }

            zones
        });

        Self {
            zones,
            stats: CpuLocal::new(|_| RefCell::default()),
        }
    }

    /// Returns null on failure.
    ///
    /// See `malloc` on the PS4 for a reference.
    ///
    /// # Safety
    /// `layout` must be nonzero.
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Our implementation imply M_WAITOK.
        let td = current_thread();

        if !td.can_sleep() {
            panic!("heap allocation in a non-sleeping context is not supported");
        }

        // Determine how to allocate.
        let size = layout.size();

        if size <= PAGE_SIZE.get() {
            // Get zone to allocate from.
            let align = layout.align().trailing_zeros() as usize;
            let size = if (size & Self::KMEM_ZMASK) != 0 {
                // TODO: Refactor this for readability.
                (size + Self::KMEM_ZBASE) & !Self::KMEM_ZMASK
            } else {
                size
            };

            // Allocate a memory from UMA zone.
            let zone = &self.zones[align][size >> Self::KMEM_ZSHIFT];
            let mem = zone.alloc();

            // Update stats.
            let stats = self.stats.lock();
            let mut stats = stats.borrow_mut();
            let size = if mem.is_null() { 0 } else { zone.size().get() };

            if size != 0 {
                stats.alloc_bytes = stats
                    .alloc_bytes
                    .checked_add(size.try_into().unwrap())
                    .unwrap();
                stats.alloc_count += 1;
            }

            // TODO: How to update mts_size here since our zone table also indexed by alignment?
            mem
        } else {
            todo!()
        }
    }

    /// # Safety
    /// `ptr` must be obtained with [`Self::alloc()`] and `layout` must be the same one that was
    /// passed to that method.
    pub unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        todo!()
    }
}

/// Implementation of `malloc_type_stats` structure.
#[derive(Default)]
struct Stats {
    alloc_bytes: u64, // mts_memalloced
    alloc_count: u64, // mts_numallocs
}
