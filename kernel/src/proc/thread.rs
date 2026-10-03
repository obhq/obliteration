use super::{LocalCell, Proc};
use alloc::sync::Arc;
use core::cell::Cell;
use core::mem::transmute;
use core::sync::atomic::{AtomicU8, Ordering};
use crossbeam_utils::CachePadded;
use sync_unsafe_cell::SyncUnsafeCell;

/// Implementation of `thread` structure.
///
/// All thread **must** run to completion once execution has been started otherwise resource will be
/// leak if the thread is dropped without finished execution.
///
/// We subtitute `TDP_NOSLEEPING` with `td_intr_nesting_level` and `td_critnest` since it is the
/// only cases the thread should not allow to sleep.
///
/// A field in the struct belong to one of the following group:
///
/// 1. Shared.
/// 2. Local accessible through [LocalState].
/// 3. Local accessible through this struct.
///
/// An interrupt handler can access only fields in group 1.
#[repr(C)] // Force all fields in group 3 to be on a different cache line from group 1.
pub struct Thread {
    proc: Arc<Proc>,             // td_proc
    active_pins: AtomicU8,       // td_critnest
    active_interrupts: AtomicU8, // td_intr_nesting_level
    local_state: CachePadded<SyncUnsafeCell<LocalState>>,
    active_mutexes: LocalCell<Cell<u16>>, // td_locks
    sleeping: LocalCell<Cell<usize>>,     // td_wchan
    active_heap_guard: LocalCell<Cell<usize>>,
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
        Self {
            proc,
            active_pins: AtomicU8::new(0),
            active_interrupts: AtomicU8::new(0),
            local_state: CachePadded::new(SyncUnsafeCell::new(LocalState { profiling_ticks: 0 })),
            active_mutexes: LocalCell::new(Cell::new(0)),
            sleeping: LocalCell::new(Cell::new(0)),
            active_heap_guard: LocalCell::new(Cell::new(0)),
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

    /// # Safety
    /// It is **very very very** easy to cause UB with this method. This method is ultra dangerous
    /// and can be safely called in a very limited location so if you think you are going to use
    /// this method in some random places, 99.99% it is wrong.
    pub unsafe fn local_state_unchecked(&self) -> &mut LocalState {
        unsafe { transmute(self.local_state.get()) }
    }

    pub fn active_mutexes(&self) -> &LocalCell<Cell<u16>> {
        &self.active_mutexes
    }

    /// Sleeping address. Zero if this thread is not in a sleep queue.
    pub fn sleeping(&self) -> &LocalCell<Cell<usize>> {
        &self.sleeping
    }

    /// # Safety
    /// This method cannot be called by the other thread or interrupt handler.
    pub unsafe fn active_heap_guard(&self) -> usize {
        unsafe { self.active_heap_guard.get() }
    }

    /// # Safety
    /// This method cannot be called by the other thread or interrupt handler.
    pub unsafe fn disable_vm_heap(&self) -> HeapGuard<'_> {
        let active = unsafe { self.active_heap_guard.as_ref() };

        active.update(|v| v.strict_add(1));

        HeapGuard(active)
    }
}

/// Contains data to be used exclusively by the execution thread.
pub struct LocalState {
    pub profiling_ticks: u32, // td_pticks
}

/// RAII struct to disable VM heap for the thread.
pub struct HeapGuard<'a>(&'a Cell<usize>);

impl Drop for HeapGuard<'_> {
    fn drop(&mut self) {
        self.0.update(|v| v - 1);
    }
}
