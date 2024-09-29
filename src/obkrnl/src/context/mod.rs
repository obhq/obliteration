use crate::proc::{ProcMgr, Thread};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicPtr, Ordering};

pub use self::arc::*;
pub use self::local::*;

mod arc;
#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod local;

/// Implementation of `pcpu` structure.
///
/// Access to this structure must be done by **atomic reading or writing its field directly**. It is
/// not safe to have a temporary a pointer or reference to this struct or its field because the CPU
/// might get interupted, which mean it is possible for the next instruction to get executed on
/// a different CPU if the interupt cause the CPU to switch the task.
///
/// The activation of this struct is a minimum requirements for a new CPU to call most of the other
/// functions. The new CPU should call [`Context::activate`] as soon as possible. We don't make the
/// functions that require this context as `unsafe` nor make it check for the context because it
/// will be (almost) all of it. So we impose this requirement on a function that setup a CPU
/// instead.
///
/// Beware for any type that implement [`Drop`] because it may access the CPU context. For maximum
/// safety the CPU setup function **must not cause any value of the kernel type to drop before
/// context is activated**. It is safe to drop values of Rust core type (e.g. `String`) **only on a
/// main CPU** because the only kernel functions it can call into is either stage 1 allocator or
/// panic handler, both of them does not require a CPU context.
#[allow(dead_code)] // All fields accessed by inline assembly.
pub struct Context {
    cpu: usize,                // pc_cpuid
    thread: AtomicPtr<Thread>, // pc_curthread
    pmgr: *const ProcMgr,
}

impl Context {
    /// Once this function return you should call [`Self::activate()`] as soon as possible. The
    /// returned value cannot be dropped otherwise it will be panic.
    ///
    /// See `pcpu_init` on the PS4 for a reference.
    ///
    /// # Context safety
    /// This function does not require a CPU context.
    ///
    /// # Safety
    /// - `cpu` must be unique and valid.
    /// - `pmgr` must be the same for all context of the other CPU.
    pub unsafe fn new(cpu: usize, td: Arc<Thread>, pmgr: Arc<ProcMgr>) -> Self {
        Self {
            cpu,
            thread: AtomicPtr::new(Arc::into_raw(td).cast_mut()),
            pmgr: Arc::into_raw(pmgr),
        }
    }

    /// # Interupt safety
    /// This function is interupt safe.
    pub fn thread() -> BorrowedArc<Thread> {
        // It does not matter if we are on a different CPU after we load the Context::thread because
        // it is going to be the same one since it represent the current thread.
        unsafe { BorrowedArc::new(self::arch::thread()) }
    }

    /// Pin the calling thread to one CPU.
    ///
    /// This thread will never switch to a different CPU until the returned [`PinnedContext`] is
    /// dropped (but it is allowed to sleep).
    ///
    /// See `critical_enter` and `critical_exit` on the PS4 for a reference.
    pub fn pin() -> PinnedContext {
        // Relax ordering should be enough here since this increment will be checked by the same CPU
        // when an interupt happens.
        let td = unsafe { self::arch::thread() };

        unsafe { (*td).critical_sections().fetch_add(1, Ordering::Relaxed) };

        PinnedContext(td)
    }

    /// # Safety
    /// The only place this method is safe to call is in the CPU entry point. Once this method
    /// return this instance must outlive the CPU lifetime and it must never be accessed via this
    /// variable again. The simple way to achieve this is keep the activated [`Context`] as a local
    /// variable then move all code after it to a dedicated no-return function.
    pub unsafe fn activate(&mut self) {
        self::arch::activate(self);
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        panic!("dropping Context can cause a bug so it is not supported");
    }
}

/// RAII struct to pin the current thread to a CPU.
///
/// This struct must not implement [`Send`] and [`Sync`]. Currently it stored a pointer, which will
/// make it `!Send` and `!Sync`.
pub struct PinnedContext(*const Thread);

impl PinnedContext {
    /// See [`CpuLocal`] for a safe alternative if you want to store per-CPU value.
    ///
    /// # Safety
    /// Anything that derive from the returned value will invalid when this [`PinnedContext`]
    /// dropped.
    pub unsafe fn cpu(&self) -> usize {
        self::arch::cpu()
    }
}

impl Drop for PinnedContext {
    fn drop(&mut self) {
        // Relax ordering should be enough here since this decrement will be checked by the same CPU
        // when an interupt happens.
        let td = unsafe { &*self.0 };

        unsafe { td.critical_sections().fetch_sub(1, Ordering::Relaxed) };

        // TODO: Implement td_owepreempt.
    }
}
