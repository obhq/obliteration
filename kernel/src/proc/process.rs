use super::{ProcAbi, ProcEvents};
use crate::event::EventSet;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Implementation of `proc` structure.
pub struct Proc {
    abi: Arc<dyn ProcAbi>, // p_sysent
    pager: AtomicUsize,
}

impl Proc {
    /// See `proc_init` and `proc_ctor` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset              |
    /// |---------|---------------------|
    /// |PS4 11.00|0x375970 and 0x3755D0|
    pub fn new(abi: Arc<dyn ProcAbi>, events: &Arc<EventSet<ProcEvents>>) -> Arc<Self> {
        let mut proc = Self {
            abi,
            pager: AtomicUsize::new(0),
        };

        // Trigger process_init event.
        let mut et = events.trigger();

        for h in et.select(|s| &s.process_init) {
            h(&mut proc);
        }

        // Trigger process_ctor event.
        let proc = Arc::new(proc);
        let weak = Arc::downgrade(&proc);

        for h in et.select(|s| &s.process_ctor) {
            h(&weak);
        }

        drop(et);

        todo!()
    }

    /// This function does not do anything except initialize the struct memory. It is the caller
    /// responsibility to configure the process after this so it have a proper states and trigger
    /// necessary events.
    ///
    /// # Context safety
    /// This function does not require a CPU context.
    pub fn new_bare(abi: Arc<dyn ProcAbi>) -> Self {
        Self {
            abi,
            pager: AtomicUsize::new(0),
        }
    }

    pub fn abi(&self) -> &Arc<dyn ProcAbi> {
        &self.abi
    }

    pub fn pager(&self) -> usize {
        self.pager.load(Ordering::Relaxed)
    }
}
