use self::vm::VmHeap;
use crate::context::current_thread;
use crate::lock::Mutex;
use alloc::boxed::Box;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::{RefCell, UnsafeCell};
use core::hint::unreachable_unchecked;
use core::ptr::{NonNull, null_mut};
use talc::{ClaimOnOom, Span, Talc};

mod vm;

/// Implementation of [`GlobalAlloc`] for objects belong to kernel space.
///
/// This allocator has 2 stages. The first stage will allocate a memory from a static buffer (AKA
/// arena). This stage will be primary used for bootstrapping the kernel. The second stage will be
/// activated once the required subsystems has been initialized.
///
/// The first stage is **not** thread safe so stage 2 must be activated before start a new CPU.
pub struct KernelHeap {
    stage: UnsafeCell<Stage>,
    primitive_ptr: *const u8,
    primitive_end: *const u8,
}

impl KernelHeap {
    /// # Safety
    /// The specified memory must be valid for reads and writes and it must be exclusively available
    /// to [`KernelHeap`].
    pub const unsafe fn new<const L: usize>(primitive: *mut [u8; L]) -> Self {
        let primitive_ptr = primitive.cast();

        // SAFETY: The safety requirement of our function satify the safety requirement of
        // ClaimOnOom::new().
        let primitive = unsafe { Talc::new(ClaimOnOom::new(Span::from_array(primitive))) };

        Self {
            stage: UnsafeCell::new(Stage::One(RefCell::new(primitive))),
            primitive_ptr,
            // SAFETY: L is a length of primitive_ptr so the resulting pointer is valid.
            primitive_end: unsafe { primitive_ptr.add(L) },
        }
    }

    /// # Safety
    /// This must be called by main CPU and can be called only once.
    pub unsafe fn activate_stage2(&self) {
        // Setup VM  heap using primitive heap.
        let vm = Box::new(VmHeap::new());

        // What we are doing here is highly unsafe. Do not edit the code after this unless you know
        // what you are doing!
        let stage = self.stage.get();
        let primitive = match unsafe { stage.read() } {
            Stage::One(v) => Mutex::new(v.into_inner()),
            // SAFETY: The safety requirement of our function make this unreachable.
            Stage::Two(_, _) => unsafe { unreachable_unchecked() },
        };

        // Switch to stage 2 WITHOUT dropping the value contained in Stage::One. What we did here is
        // moving the value from Stage::One to Stage::Two.
        unsafe { stage.write(Stage::Two(vm, primitive)) };
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    #[inline(never)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.

        // SAFETY: GlobalAlloc::alloc required layout to be non-zero.
        match unsafe { &*self.stage.get() } {
            Stage::One(primitive) => unsafe {
                primitive
                    .borrow_mut()
                    .malloc(layout)
                    .map_or(null_mut(), |v| v.as_ptr())
            },
            Stage::Two(vm, primitive) => match current_thread().active_heap_guard() {
                0 => unsafe { vm.alloc(layout) },
                _ => unsafe {
                    primitive
                        .lock()
                        .malloc(layout)
                        .map_or(null_mut(), |v| v.as_ptr())
                },
            },
        }
    }

    #[inline(never)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // If stage 2 has not activated yet then this function is not allowed to access the CPU
        // context due to it can be called before the context has been activated.

        // SAFETY: GlobalAlloc::dealloc required ptr to be the same one that returned
        // from our GlobalAlloc::alloc and layout to be the same one that passed to it.
        match unsafe { &*self.stage.get() } {
            Stage::One(primitive) => unsafe {
                primitive
                    .borrow_mut()
                    .free(NonNull::new_unchecked(ptr), layout)
            },
            Stage::Two(vm, primitive) => {
                if ptr.cast_const() >= self.primitive_ptr && ptr.cast_const() < self.primitive_end {
                    unsafe { primitive.lock().free(NonNull::new_unchecked(ptr), layout) }
                } else {
                    // SAFETY: ptr is not owned by primitive heap so with the requirements of
                    // GlobalAlloc::dealloc the ptr will be owned by VM heap for sure.
                    unsafe { vm.dealloc(ptr, layout) };
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
    Two(Box<VmHeap>, Mutex<Talc<ClaimOnOom>>),
}
