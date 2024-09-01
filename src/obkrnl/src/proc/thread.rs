/// Implementation of `thread` structure.
///
/// All thread **must** run to completion once execution has been started otherwise resource will be
/// leak if the thread is dropped while its execution currently in the kernel space.
pub struct Thread {
    active_interrupts: usize, // td_intr_nesting_level
}

impl Thread {
    /// # Safety
    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the thread after this so it have a proper states and trigger
    /// necessary events.
    pub unsafe fn new_bare() -> Self {
        Self {
            active_interrupts: 0,
        }
    }

    pub fn active_interrupts(&self) -> usize {
        self.active_interrupts
    }
}
