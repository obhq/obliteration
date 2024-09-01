use self::stage1::Stage1;
use self::stage2::Stage2;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

mod stage1;
mod stage2;

/// Implementation of [`GlobalAlloc`] for objects belong to kernel space.
///
/// This allocator has 2 stages. The first stage will allocate a memory from a static buffer (AKA
/// arena). This stage will be primary used for bootstrapping the kernel. The second stage will be
/// activated once the required subsystems has been initialized.
pub struct KernelHeap {
    stage1: Stage1,
    stage2: AtomicPtr<Stage2>,
}

impl KernelHeap {
    /// # Safety
    /// The specified memory must be valid for reads and writes and it must be exclusively available
    /// to [`KernelHeap`].
    pub const unsafe fn new(stage1: *mut u8, len: usize) -> Self {
        Self {
            stage1: Stage1::new(stage1, len),
            stage2: AtomicPtr::new(null_mut()),
        }
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let stage2 = self.stage2.load(Ordering::Relaxed);

        if stage2.is_null() {
            // SAFETY: GlobalAlloc::alloc required layout to be non-zero.
            self.stage1.alloc(layout)
        } else {
            todo!()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if self.stage1.is_owner(ptr) {
            // SAFETY: GlobalAlloc::dealloc required ptr to be the same one that returned from our
            // GlobalAlloc::alloc and layout to be the same one that passed to it.
            self.stage1.dealloc(ptr, layout);
        } else {
            todo!()
        }
    }
}
