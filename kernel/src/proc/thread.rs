use super::Proc;
use super::cell::{PrivateCell, get, set};
use crate::lock::{Gutex, GutexGroup, GutexWrite};
use alloc::sync::Arc;
use core::cell::Cell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU8, Ordering};

/// Implementation of `thread` structure.
///
/// All thread **must** run to completion once execution has been started otherwise resource will be
/// leak if the thread is dropped while its execution currently in the kernel space.
///
/// We subtitute `TDP_NOSLEEPING` with `td_intr_nesting_level` and `td_critnest` since it is the
/// only cases the thread should not allow to sleep.
///
/// Do not try to access any [RefCell](core::cell::RefCell) fields from interrupt handler because it
/// might currently locked.
pub struct Thread {
    proc: Arc<Proc>,                         // td_proc
    active_pins: AtomicU8,                   // td_critnest
    active_interrupts: AtomicU8,             // td_intr_nesting_level
    active_mutexes: PrivateCell<Cell<u16>>,  // td_locks
    sleeping: Gutex<usize>,                  // td_wchan
    profiling_ticks: PrivateCell<Cell<u32>>, // td_pticks
    active_heap_guard: PrivateCell<Cell<usize>>,
}

impl Thread {
    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the thread after this so it have a proper states and trigger
    /// necessary events.
    ///
    /// # Context safety
    /// This function does not require a CPU context.
    pub fn new_bare(proc: Arc<Proc>) -> Self {
        // td_critnest on the PS4 started with 1 but this does not work in our case because we use
        // RAII to increase and decrease it.
        let gg = GutexGroup::new();

        Self {
            proc,
            active_pins: AtomicU8::new(0),
            active_interrupts: AtomicU8::new(0),
            active_mutexes: PrivateCell::default(),
            sleeping: gg.spawn(0),
            profiling_ticks: PrivateCell::default(),
            active_heap_guard: PrivateCell::default(),
        }
    }

    pub fn can_sleep(&self) -> bool {
        // Both of the values here can only modified by this thread so no race condition here.
        let active_pins = self.active_pins.load(Ordering::Relaxed);
        let active_interrupts = self.active_interrupts.load(Ordering::Relaxed);

        active_pins == 0 && active_interrupts == 0
    }

    pub fn proc(&self) -> &Arc<Proc> {
        &self.proc
    }

    /// See [`crate::context::pin_cpu()`] for a safe wrapper.
    ///
    /// # Safety
    /// Once this value is zero this thread can switch to a different CPU. The code after this value
    /// decrement must not depend on a specific CPU.
    ///
    /// This value must not modified by the other thread.
    pub unsafe fn active_pins(&self) -> &AtomicU8 {
        &self.active_pins
    }

    /// # Safety
    /// This value can only modified by interrupt entry point.
    pub unsafe fn active_interrupts(&self) -> &AtomicU8 {
        &self.active_interrupts
    }

    /// # Panics
    /// If called from the other thread.
    pub fn active_mutexes(&self) -> u16 {
        get!(self, active_mutexes)
    }

    /// # Panics
    /// If called from the other thread.
    pub fn set_active_mutexes(&self, v: u16) {
        set!(self, active_mutexes, v)
    }

    /// Sleeping address. Zero if this thread is not in a sleep queue.
    pub fn sleeping_mut(&self) -> GutexWrite<'_, usize> {
        self.sleeping.write()
    }

    /// # Panics
    /// If called from the other thread.
    pub fn set_profiling_ticks(&self, v: u32) {
        set!(self, profiling_ticks, v)
    }

    /// # Panics
    /// If called from the other thread.
    pub fn active_heap_guard(&self) -> usize {
        get!(self, active_heap_guard)
    }

    pub fn disable_vm_heap(&self) -> HeapGuard<'_> {
        let v = get!(self, active_heap_guard).checked_add(1).unwrap();

        set!(self, active_heap_guard, v);

        HeapGuard {
            td: self,
            phantom: PhantomData,
        }
    }
}

/// RAII struct to disable VM heap for the thread.
pub struct HeapGuard<'a> {
    td: &'a Thread,
    phantom: PhantomData<*const ()>, // For !Send and !Sync.
}

impl Drop for HeapGuard<'_> {
    fn drop(&mut self) {
        let td = self.td;
        let v = get!(td, active_heap_guard) - 1;

        set!(td, active_heap_guard, v);
    }
}
