use crate::pcpu::Pcpu;
use crate::process::VThread;
use std::sync::Arc;

pub use self::runq::*;

mod runq;

/// Threads scheduler.
pub struct Scheduler {
    rq: Runq,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { rq: Runq::new() }
    }

    /// See `sched_choose` on the PS4 for a reference.
    pub fn choose(&self, cx: &Pcpu) -> Arc<VThread> {
        // TODO: Pull a ready thread from runq.
        cx.idle_thread().upgrade().unwrap()
    }
}
