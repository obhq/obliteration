use super::{ProcAbi, ProcEvents};
use crate::event::EventSet;
use alloc::sync::Arc;

/// Implementation of `proc` structure.
pub struct Proc {
    abi: Arc<dyn ProcAbi>, // p_sysent
}

impl Proc {
    pub fn new(abi: Arc<dyn ProcAbi>, events: &Arc<EventSet<ProcEvents>>) -> Arc<Self> {
        let mut proc = Self { abi };

        // Trigger process_init event.
        let mut et = events.trigger();

        for h in et.select(|s| &s.process_init) {
            h(&mut proc);
        }

        todo!()
    }

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
