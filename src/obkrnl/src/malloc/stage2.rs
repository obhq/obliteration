use crate::config::PAGE_SIZE;
use crate::context::Context;
use crate::uma::UmaZone;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::alloc::Layout;

/// Stage 2 kernel heap.
///
/// This stage allocate a memory from a virtual memory management system. This struct is a merge of
/// `malloc_type` and `malloc_type_internal` structure.
pub struct Stage2 {
    zones: Vec<Arc<UmaZone>>, // kmemsize + kmemzones
}

impl Stage2 {
    const KMEM_ZSHIFT: usize = 4;
    const KMEM_ZBASE: usize = 16;
    const KMEM_ZMASK: usize = Self::KMEM_ZBASE - 1;
    const KMEM_ZSIZE: usize = PAGE_SIZE >> Self::KMEM_ZSHIFT;

    /// See `kmeminit` on the PS4 for a reference.
    pub fn new() -> Self {
        let mut zones = Vec::with_capacity(Self::KMEM_ZSIZE + 1);
        let mut last = 0;

        for i in Self::KMEM_ZSHIFT.. {
            // Stop if size larger than page size.
            let size = 1usize << i;

            if size > PAGE_SIZE {
                break;
            }

            // Create zone.
            let zone = Arc::new(UmaZone::new(size.to_string().into(), size));

            while last <= size {
                zones.push(zone.clone());
                last += Self::KMEM_ZBASE;
            }
        }

        Self { zones }
    }

    /// See `malloc` on the PS4 for a reference.
    ///
    /// # Safety
    /// `layout` must be nonzero.
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Our implementation imply M_WAITOK.
        let td = Context::thread();

        if td.active_interrupts() != 0 {
            panic!("heap allocation in an interrupt handler is not supported");
        }

        // TODO: Handle alignment.
        let mut size = layout.size();

        if size <= PAGE_SIZE {
            // Round size.
            if (size & Self::KMEM_ZMASK) != 0 {
                // TODO: Refactor this for readability.
                size = (size + Self::KMEM_ZBASE) & !Self::KMEM_ZMASK;
            }

            // TODO: There are more logic after this on the PS4.
            self.zones[size >> Self::KMEM_ZSHIFT].alloc()
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
