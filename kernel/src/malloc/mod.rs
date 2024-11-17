pub use self::stage2::Stage2;
use crate::lock::Mutex;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::{RefCell, UnsafeCell};
use core::hint::unreachable_unchecked;
use core::ptr::{null_mut, NonNull};
use talc::{ClaimOnOom, Span, Talc};

mod stage2;

/// Implementation of [`GlobalAlloc`] for objects belong to kernel space.
///
/// This allocator has 2 stages. The first stage will allocate a memory from a static buffer (AKA
/// arena). This stage will be primary used for bootstrapping the kernel. The second stage will be
/// activated once the required subsystems has been initialized.
///
/// The first stage is **not** thread safe so stage 2 must be activated before start a new CPU.
pub struct KernelHeap {
    stage: UnsafeCell<Stage>,
    stage1_ptr: *const u8,
    stage1_end: *const u8,
}

impl KernelHeap {
    /// # Safety
    /// The specified memory must be valid for reads and writes and it must be exclusively available
    /// to [`KernelHeap`].
    pub const unsafe fn new<const L: usize>(stage1: *mut [u8; L]) -> Self {
        let stage1_ptr = stage1.cast();
        let stage1 = Talc::new(ClaimOnOom::new(Span::from_array(stage1)));

        Self {
            stage: UnsafeCell::new(Stage::One(RefCell::new(stage1))),
            stage1_ptr,
            stage1_end: stage1_ptr.add(L),
        }
    }

    /// # Safety
    /// This must be called by main CPU and can be called only once.
    pub unsafe fn activate_stage2(&self, stage2: Stage2) {
        // What we are going here is highly unsafe. Do not edit this code unless you know what you
        // are doing!
        let stage = self.stage.get();
        let stage1 = match stage.read() {
            Stage::One(v) => Mutex::new(v.into_inner()),
            Stage::Two(_, _) => unreachable_unchecked(),
        };

        // Switch to stage 2 WITHOUT dropping the value contained in Stage::One.
        stage.write(Stage::Two(stage2, stage1));
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    #[inline(never)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.
        // SAFETY: GlobalAlloc::alloc required layout to be non-zero.
        match &*self.stage.get() {
            Stage::One(s) => s
                .borrow_mut()
                .malloc(layout)
                .map(|v| v.as_ptr())
                .unwrap_or(null_mut()),
            Stage::Two(s, _) => s.alloc(layout),
        }
    }

    #[inline(never)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.
        match &*self.stage.get() {
            Stage::One(s) => s.borrow_mut().free(NonNull::new_unchecked(ptr), layout),
            Stage::Two(s2, s1) => {
                if ptr.cast_const() >= self.stage1_ptr && ptr.cast_const() < self.stage1_end {
                    // SAFETY: GlobalAlloc::dealloc required ptr to be the same one that returned
                    // from our GlobalAlloc::alloc and layout to be the same one that passed to it.
                    s1.lock().free(NonNull::new_unchecked(ptr), layout)
                } else {
                    // SAFETY: ptr is not owned by stage 1 so with the requirements of
                    // GlobalAlloc::dealloc the pr will be owned by stage 2 for sure.
                    s2.dealloc(ptr, layout);
                }
            }
        }
    }
}

// We impose restriction on the user to activate stage 2 before going multi-threaded.
unsafe impl Send for KernelHeap {}
unsafe impl Sync for KernelHeap {}

/// Stage of [KernelHeap].
enum Stage {
    One(RefCell<Talc<ClaimOnOom>>),
    Two(Stage2, Mutex<Talc<ClaimOnOom>>),
}
