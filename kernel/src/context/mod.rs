pub use self::arc::*;
pub use self::local::*;

use self::arch::{load_fixed_ptr, load_usize};
use crate::proc::{ProcMgr, Thread};
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::marker::PhantomData;
use core::mem::offset_of;

mod arc;
#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod local;

/// See `pcpu_init` on the PS4 for a reference.
///
/// # Context safety
/// This function does not require a CPU context.
///
/// # Safety
/// - This function can be called only once per CPU.
/// - `cpu` must be unique and valid.
/// - `pmgr` must be the same for all context.
/// - `f` must not get inlined.
pub unsafe fn run_with_context(cpu: usize, td: Arc<Thread>, pmgr: Arc<ProcMgr>, f: fn() -> !) -> ! {
    // We use a different mechanism here. The PS4 put all of pcpu at a global level but we put it on
    // each CPU stack instead.
    let mut cx = self::arch::Context::new(Context {
        cpu,
        thread: Arc::into_raw(td),
        pmgr: Arc::into_raw(pmgr),
    });

    self::arch::activate(&mut cx);
    f();
}

/// Implementation of `pcpu` structure.
///
/// Access to this structure must be done by **atomic reading or writing its field directly**. It is
/// not safe to have a temporary a pointer or reference to this struct or its field because the CPU
/// might get interupted, which mean it is possible for the next instruction to get executed on
/// a different CPU if the interupt cause the CPU to switch the task.
///
/// The activation of this struct is a minimum requirements for a new CPU to call most of the other
/// functions. The new CPU should call [`run_with_context()`] as soon as possible. We don't make the
/// functions that require this context as `unsafe` nor make it check for the context because it
/// will be (almost) all of it. So we impose this requirement on a function that setup a CPU
/// instead.
///
/// Beware for any type that implement [`Drop`] because it may access the CPU context. For maximum
/// safety the CPU setup function **must not cause any value of the kernel type to drop before
/// context is activated**. It is safe to drop values of Rust core type (e.g. `String`) **only on a
/// main CPU** because the only kernel functions it can call into is either stage 1 allocator or
/// panic handler, both of them does not require a CPU context.
#[repr(C)]
pub struct Context {
    cpu: usize,            // pc_cpuid
    thread: *const Thread, // pc_curthread
    pmgr: *const ProcMgr,
}

impl Context {
    /// # Interupt safety
    /// This function is interupt safe.
    pub fn thread() -> BorrowedArc<Thread> {
        // It does not matter if we are on a different CPU after we load the Context::thread because
        // it is going to be the same one since it represent the current thread.
        unsafe { BorrowedArc::new(load_fixed_ptr::<{ offset_of!(Self, thread) }, _>()) }
    }

    pub fn procs() -> BorrowedArc<ProcMgr> {
        // It does not matter if we are on a different CPU after we load the Context::pmgr because
        // it is always the same for all CPU.
        unsafe { BorrowedArc::new(load_fixed_ptr::<{ offset_of!(Self, pmgr) }, _>()) }
    }

    /// Pin the calling thread to one CPU.
    ///
    /// This thread will never switch to a different CPU until the returned [`PinnedContext`] is
    /// dropped and it is not allowed to sleep.
    ///
    /// See `critical_enter` and `critical_exit` on the PS4 for a reference. Beware that our
    /// implementation a bit different. The PS4 **allow the thread to sleep but we don't**.
    pub fn pin() -> PinnedContext {
        let td = Self::thread();

        unsafe { *td.critical_sections_mut() += 1 };

        PinnedContext {
            td,
            phantom: PhantomData,
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        panic!("dropping Context can cause a bug so it is not supported");
    }
}

/// RAII struct to pin the current thread to a CPU.
///
/// This struct must not implement [`Send`] and [`Sync`].
pub struct PinnedContext {
    td: BorrowedArc<Thread>,
    phantom: PhantomData<Rc<()>>, // For !Send and !Sync.
}

impl PinnedContext {
    /// See [`CpuLocal`] for a safe alternative if you want to store per-CPU value.
    ///
    /// # Safety
    /// Anything that derive from the returned value will invalid when this [`PinnedContext`]
    /// dropped.
    pub unsafe fn cpu(&self) -> usize {
        load_usize::<{ offset_of!(Context, cpu) }>()
    }
}

impl Drop for PinnedContext {
    fn drop(&mut self) {
        unsafe { *self.td.critical_sections_mut() -= 1 };

        // TODO: Implement td_owepreempt.
    }
}
