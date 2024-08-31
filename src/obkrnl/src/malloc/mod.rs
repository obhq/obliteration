use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null_mut, NonNull};
use talc::{ClaimOnOom, Span, Talc};

/// Implementation of [`GlobalAlloc`] for objects belong to kernel space.
///
/// This allocator has 2 stages. The first stage will allocate a memory from a static buffer (AKA
/// arena). This stage will be primary used for bootstrapping the kernel. The second stage will be
/// activated once the required subsystems has been initialized.
pub struct KernelHeap {
    stage1: spin::Mutex<Talc<ClaimOnOom>>,
}

impl KernelHeap {
    /// # Safety
    /// The specified memory must be valid for reads and writes and it must be exclusively available
    /// to [`KernelHeap`].
    pub const unsafe fn new(stage1: *mut u8, len: usize) -> Self {
        let stage1 = Talc::new(unsafe { ClaimOnOom::new(Span::from_base_size(stage1, len)) });

        Self {
            stage1: spin::Mutex::new(stage1),
        }
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc::alloc required layout to be non-zero.
        self.stage1
            .lock()
            .malloc(layout)
            .map(|v| v.as_ptr())
            .unwrap_or(null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: GlobalAlloc::dealloc required ptr to be the same one that returned from our
        // GlobalAlloc::alloc and layout to be the same one that passed to it.
        self.stage1.lock().free(NonNull::new_unchecked(ptr), layout);
    }
}
