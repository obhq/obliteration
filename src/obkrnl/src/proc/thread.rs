/// Implementation of `thread` structure.
pub struct Thread {}

impl Thread {
    /// # Safety
    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the thread after this so it have a proper states and trigger
    /// necessary events.
    pub unsafe fn new_bare() -> Self {
        Self {}
    }
}
