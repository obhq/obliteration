use crate::context::{BorrowedArc, Context};
use crate::lock::{Gutex, GutexGroup, GutexWriteGuard};
use core::cell::{RefCell, RefMut};
use core::sync::atomic::AtomicU16;

/// Implementation of `thread` structure.
///
/// All thread **must** run to completion once execution has been started otherwise resource will be
/// leak if the thread is dropped while its execution currently in the kernel space.
///
/// We subtitute `TDP_NOSLEEPING` with `td_intr_nesting_level` since the only cases the thread
/// should not allow to sleep is when it being handle an interupt.
pub struct Thread {
    critical_sections: Private<u32>, // td_critnest
    active_interrupts: usize,        // td_intr_nesting_level
    active_mutexes: AtomicU16,       // td_locks
    sleeping: Gutex<usize>,          // td_wchan
}

impl Thread {
    /// # Context safety
    /// This function does not require a CPU context.
    ///
    /// # Safety
    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the thread after this so it have a proper states and trigger
    /// necessary events.
    pub unsafe fn new_bare() -> Self {
        // td_critnest on the PS4 started with 1 but this does not work in our case because we use
        // RAII to increase and decrease it.
        let gg = GutexGroup::new();

        Self {
            critical_sections: Private::new(0),
            active_interrupts: 0,
            active_mutexes: AtomicU16::new(0),
            sleeping: gg.spawn(0),
        }
    }

    /// See [`crate::context::Context::pin()`] for a safe wrapper.
    ///
    /// # Safety
    /// This is a counter. Each increment must paired with a decrement. Failure to do so will cause
    /// the whole system to be in an undefined behavior.
    ///
    /// # Panics
    /// If called from the other thread.
    pub unsafe fn critical_sections_mut(&self) -> RefMut<u32> {
        self.critical_sections.borrow_mut(self)
    }

    pub fn active_interrupts(&self) -> usize {
        self.active_interrupts
    }

    pub fn active_mutexes(&self) -> &AtomicU16 {
        &self.active_mutexes
    }

    /// Sleeping address. Zero if this thread is not in a sleep queue.
    pub fn sleeping_mut(&self) -> GutexWriteGuard<usize> {
        self.sleeping.write()
    }
}

/// Encapsulates a field of [Thread] that can only be accessed by the CPU that currently executing
/// the thread.
struct Private<T>(RefCell<T>);

impl<T> Private<T> {
    fn new(v: T) -> Self {
        Self(RefCell::new(v))
    }

    fn borrow_mut(&self, td: &Thread) -> RefMut<T> {
        self.validate(td);
        self.0.borrow_mut()
    }

    fn validate(&self, td: &Thread) {
        let current = Context::thread();

        if !core::ptr::eq(BorrowedArc::as_ptr(&current), td) {
            panic!("accessing a private field from the other thread is not supported");
        }
    }
}
