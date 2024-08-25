use core::alloc::{GlobalAlloc, Layout};

/// Implementation of [`GlobalAlloc`] for objects belong to kernel space.
pub struct KernelHeap {}

impl KernelHeap {
    pub const fn new() -> Self {
        Self {}
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        todo!()
    }
}
