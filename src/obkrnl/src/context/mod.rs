use crate::proc::Thread;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicPtr, Ordering};

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

/// Implementation of `pcpu` structure.
///
/// Access to this structure must be done by **atomic reading or writing its field directly**. It is
/// not safe to have a temporary a pointer or reference to this struct or its field because the CPU
/// might get interupted, which mean it is possible for the next instruction to get executed on
/// a different CPU if the interupt cause the CPU to switch the task.
pub struct Context {
    cpu: usize,                // pc_cpuid
    thread: AtomicPtr<Thread>, // pc_curthread
}

impl Context {
    /// See `pcpu_init` on the PS4 for a reference.
    pub fn new(cpu: usize, td: Arc<Thread>) -> Self {
        Self {
            cpu,
            thread: AtomicPtr::new(Arc::into_raw(td).cast_mut()),
        }
    }

    /// # Interupt safety
    /// This function is interupt safe.
    #[inline(never)]
    pub fn thread() -> Arc<Thread> {
        // It does not matter if we are on a different CPU after we load the Context::thread because
        // it is going to be the same one since it represent the current thread.
        let td = unsafe { self::arch::thread() };

        // We cannot return a reference here because it requires 'static lifetime, which allow the
        // caller to store it at a global level. Once the thread is destroyed that reference will be
        // invalid.
        unsafe { Arc::increment_strong_count(td) };
        unsafe { Arc::from_raw(td) }
    }

    /// Pin the calling thread to one CPU.
    ///
    /// This thread will never switch to a different CPU until the returned [`PinnedContext`] is
    /// dropped (but it is allowed to sleep).
    ///
    /// See `critical_enter` and `critical_exit` on the PS4 for a reference.
    #[inline(never)]
    pub fn pin() -> PinnedContext {
        // Relax ordering should be enough here since this increment will be checked by the same CPU
        // when an interupt happens.
        let td = unsafe { self::arch::thread() };

        unsafe { (*td).critical_sections().fetch_add(1, Ordering::Relaxed) };

        // Once the thread is in a critical section it will never be switch a CPU so it is safe to
        // keep a pointer to a context here.
        PinnedContext(unsafe { self::arch::current() })
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
        unsafe { drop(Arc::from_raw(self.thread.load(Ordering::Relaxed))) };
    }
}

/// RAII struct to pin the current thread to current CPU.
///
/// This struct must not implement [`Send`] and [`Sync`]. Currently it stored a pointer, which will
/// make it `!Send` and `!Sync`.
pub struct PinnedContext(*const Context);

impl PinnedContext {
    pub fn cpu(&self) -> usize {
        unsafe { (*self.0).cpu }
    }
}

impl Drop for PinnedContext {
    fn drop(&mut self) {
        // Relax ordering should be enough here since this decrement will be checked by the same CPU
        // when an interupt happens.
        let td = unsafe { (*self.0).thread.load(Ordering::Relaxed) };

        unsafe { (*td).critical_sections().fetch_sub(1, Ordering::Relaxed) };

        // TODO: Implement td_owepreempt.
    }
}
