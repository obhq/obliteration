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
///
/// We don't support `pc_cpuid` field here because it value is 100% unpredictable due to the above
/// reason. Once we have loaded `pc_cpuid` the next instruction might get executed on a different
/// CPU, which render the loaded value incorrect. The only way to prevent this issue is to disable
/// interupt before reading `pc_cpuid`, which can make the CPU missed some events from the other
/// hardwares.
pub struct Context {
    thread: AtomicPtr<Thread>, // pc_curthread
}

impl Context {
    /// See `pcpu_init` on the PS4 for a reference.
    pub fn new(td: Arc<Thread>) -> Self {
        Self {
            thread: AtomicPtr::new(Arc::into_raw(td).cast_mut()),
        }
    }

    pub fn thread() -> Arc<Thread> {
        // It does not matter if we are on a different CPU after we load the Context::thread because
        // it is going to be the same one since it represent the current thread.
        let td = unsafe { self::arch::thread() };

        unsafe { Arc::increment_strong_count(td) };
        unsafe { Arc::from_raw(td) }
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
