use self::stage1::Stage1;
use self::stage2::Stage2;
use alloc::boxed::Box;
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
    pub const unsafe fn new<const L: usize>(stage1: *mut [u8; L]) -> Self {
        Self {
            stage1: Stage1::new(stage1),
            stage2: AtomicPtr::new(null_mut()),
        }
    }

    /// # Panics
    /// If stage 2 already activated.
    pub fn activate_stage2(&self) {
        // Setup stage 2.
        let state2 = Box::new(Stage2::new());

        // Activate.
        let state2 = Box::into_raw(state2);

        assert!(self
            .stage2
            .compare_exchange(null_mut(), state2, Ordering::Release, Ordering::Relaxed)
            .is_ok());
    }
}

impl Drop for KernelHeap {
    fn drop(&mut self) {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.
        let stage2 = self.stage2.load(Ordering::Acquire);

        if !stage2.is_null() {
            drop(unsafe { Box::from_raw(stage2) });
        }
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    #[inline(never)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.
        // SAFETY: GlobalAlloc::alloc required layout to be non-zero.
        self.stage2
            .load(Ordering::Acquire)
            .as_ref()
            .map(|stage2| stage2.alloc(layout))
            .unwrap_or_else(|| self.stage1.alloc(layout))
    }

    #[inline(never)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.
        if self.stage1.is_owner(ptr) {
            // SAFETY: GlobalAlloc::dealloc required ptr to be the same one that returned from our
            // GlobalAlloc::alloc and layout to be the same one that passed to it.
            self.stage1.dealloc(ptr, layout);
        } else {
            // SAFETY: ptr is not owned by stage 1 so with the requirements of GlobalAlloc::dealloc
            // the pr will be owned by stage 2 for sure.
            self.stage2
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .dealloc(ptr, layout);
        }
    }
}
