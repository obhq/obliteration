use linked_list_allocator::Heap;
use std::alloc::Layout;
use std::collections::BTreeMap;
use std::ptr::NonNull;
use thiserror::Error;

/// Manage all page allocations.
pub(super) struct Allocator {
    heap: Heap,
    align: usize,
    allocations: BTreeMap<usize, AllocInfo>,
}

impl Allocator {
    pub fn new(ptr: *mut u8, len: usize, align: usize) -> Self {
        Self {
            heap: unsafe { Heap::new(ptr, len) },
            align,
            allocations: BTreeMap::new(),
        }
    }

    pub fn alloc(&mut self, len: usize) -> Result<NonNull<u8>, AllocError> {
        // Allocate from heap.
        let layout = Layout::from_size_align(len, self.align).unwrap();
        let before = self.heap.used();
        let ptr = match self.heap.allocate_first_fit(layout) {
            Ok(v) => v,
            Err(_) => return Err(AllocError::NoMem),
        };

        // Make sure our allocator did the right thing.
        // FIXME: Remove this check once we sure the allocator always did the right thing.
        let addr = ptr.as_ptr() as usize;
        let len = self.heap.used() - before;

        if addr % self.align != 0 || len % self.align != 0 {
            panic!("The memory allocator returned unaligned allocation.");
        }

        // Store allocation information.
        // FIXME: Remove occupied check once we sure the allocator always did the right thing.
        if self.allocations.range(addr..(addr + len)).count() != 0 {
            panic!("The memory allocator returned an occupied allocation.");
        }

        self.allocations.insert(addr, AllocInfo { len, layout });

        Ok(ptr)
    }

    pub fn free(&mut self, ptr: NonNull<u8>) -> Result<(), FreeError> {
        use std::collections::btree_map::Entry;

        // Get allocation information.
        let addr = ptr.as_ptr() as usize;
        let info = match self.allocations.entry(addr) {
            Entry::Vacant(_) => return Err(FreeError::InvalidPtr),
            Entry::Occupied(e) => e.remove(),
        };

        // Dealloc from heap.
        unsafe { self.heap.deallocate(ptr, info.layout) };

        Ok(())
    }
}

/// Contains information for an allocation of virtual pages.
pub(super) struct AllocInfo {
    len: usize, // Actual size of allocation.
    layout: Layout,
}

/// Errors for [`alloc()`][Allocator::alloc].
#[derive(Debug, Error)]
pub enum AllocError {
    #[error("insufficient memory")]
    NoMem,
}

/// Errors for [`free()`][Allocator::free].
#[derive(Debug, Error)]
pub enum FreeError {
    #[error("invalid pointer")]
    InvalidPtr,
}
