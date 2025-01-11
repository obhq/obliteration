use crate::process::VThread;
use std::collections::VecDeque;
use std::sync::{Mutex, Weak};

/// Implementation of `runq` structure.
pub struct Runq {
    queues: [Mutex<VecDeque<Weak<VThread>>>; 1024], // rq_queues
}

impl Runq {
    pub(super) fn new() -> Self {
        Self {
            queues: [const { Mutex::new(VecDeque::new()) }; 1024],
        }
    }
}
