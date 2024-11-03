use crate::subsystem::Subsystem;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use self::ty::*;

mod ty;

pub struct Event<T: EventType> {
    next_id: u64,
}

impl<T: EventType> Event<T> {
    /// See `eventhandler_register` on the PS4 for a reference.
    ///
    /// `handler` must not call into any function that going to trigger any event in the same
    /// [`EventSet`] because it can cause a deadlock. That mean what `handler` can do is limited.
    pub fn subscribe<S: Subsystem>(
        &mut self,
        subsys: &Arc<S>,
        handler: T::Handler<S>,
        priority: u32,
    ) {
        let subsys = subsys.clone();
        let id = self.next_id;

        assert!(self
            .subscribers
            .insert((priority, id), T::wrap_handler(subsys, handler))
            .is_none());

        self.next_id += 1;
    }
}

impl<T: EventType> Default for Event<T> {
    fn default() -> Self {
        Self { next_id: 0 }
    }
}

impl<S> EventSet<S> {
    pub fn lock(&self) -> RwLockWriteGuard<S> {
        self.0.write().unwrap()
    }
}
