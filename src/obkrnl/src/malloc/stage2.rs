use crate::context::Context;
use core::alloc::Layout;

/// Stage 2 kernel heap.
///
/// This stage allocate a memory from a virtual memory management system. This struct is a merge of
/// `malloc_type` and `malloc_type_internal` structure.
pub struct Stage2 {}

impl Stage2 {
    pub fn new() -> Self {
        Self {}
    }

    /// See `malloc` on the PS4 for a reference.
    ///
    /// # Safety
    /// `layout` must be nonzero.
    pub unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        // Our implementation imply M_WAITOK.
        let td = Context::thread();

        if td.active_interrupts() != 0 {
            panic!("heap allocation in an interrupt handler is not supported");
        }

        todo!()
    }

    /// # Safety
    /// `ptr` must be obtained with [`Self::alloc()`] and `layout` must be the same one that was
    /// passed to that method.
    pub unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        todo!()
    }
}
