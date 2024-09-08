use core::sync::atomic::AtomicU32;

/// Implementation of `thread` structure.
///
/// All thread **must** run to completion once execution has been started otherwise resource will be
/// leak if the thread is dropped while its execution currently in the kernel space.
pub struct Thread {
    critical_sections: AtomicU32, // td_critnest
    active_interrupts: usize,     // td_intr_nesting_level
}

impl Thread {
    /// # Safety
    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the thread after this so it have a proper states and trigger
    /// necessary events.
    pub unsafe fn new_bare() -> Self {
        // td_critnest on the PS4 started with 1 but this does not work in our case because we use
        // RAII to increase and decrease it.
        Self {
            critical_sections: AtomicU32::new(0),
            active_interrupts: 0,
        }
    }

    pub fn critical_sections(&self) -> &AtomicU32 {
        &self.critical_sections
    }

    pub fn active_interrupts(&self) -> usize {
        self.active_interrupts
    }
}
