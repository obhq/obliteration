use crate::process::VThread;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Weak;

/// Implementation of `pcpu` structure.
pub struct Pcpu {
    id: usize,                    // pc_cpuid
    idle_thread: Weak<VThread>,   // pc_idlethread
    phantom: PhantomData<Rc<()>>, // For !Send and !Sync.
}

impl Pcpu {
    pub fn new(id: usize, idle_thread: Weak<VThread>) -> Self {
        Self {
            id,
            idle_thread,
            phantom: PhantomData,
        }
    }

    pub fn idle_thread(&self) -> &Weak<VThread> {
        &self.idle_thread
    }
}
