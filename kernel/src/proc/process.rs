use super::ProcAbi;
use alloc::sync::Arc;

/// Implementation of `proc` structure.
pub struct Proc {
    abi: Arc<dyn ProcAbi>, // p_sysent
}

impl Proc {
    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the process after this so it have a proper states and trigger
    /// necessary events.
    ///
    /// # Context safety
    /// This function does not require a CPU context.
    pub fn new_bare(abi: Arc<dyn ProcAbi>) -> Self {
        Self { abi }
    }

    pub fn abi(&self) -> &Arc<dyn ProcAbi> {
        &self.abi
    }
}
