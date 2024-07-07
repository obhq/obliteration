use crate::subsystem::Subsystem;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use self::ty::*;

mod ty;

/// Implementation of `eventhandler_list` structure.
///
/// Our implementation is different from PS4 version to make it idomatic to Rust.
pub struct Event<T: EventType> {
    subscribers: BTreeMap<(u32, u64), T::Wrapper>, // el_entries
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
        Self {
            subscribers: BTreeMap::new(),
            next_id: 0,
        }
    }
}

/// Encapsulate a set of [`Event`].
///
/// Usually there are only one [`EventSet`] per subsystem. The purpose of this struct is to prevent
/// race condition during subscribing and triggering multiple events. In other words, this struct
/// provide atomicity for subscription to multiple events in the set.
pub struct EventSet<S>(RwLock<S>);

impl<S> EventSet<S> {
    pub fn lock(&self) -> RwLockWriteGuard<S> {
        self.0.write().unwrap()
    }

    pub fn trigger(&self) -> EventTrigger<S> {
        EventTrigger(self.0.read().unwrap())
    }
}

impl<S: Default> Default for EventSet<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Struct to trigger one or more events.
///
/// It is guarantee that no other handler in the current set will get registered until this struct
/// has been dropped.
pub struct EventTrigger<'a, S>(RwLockReadGuard<'a, S>);

impl<'a, S> EventTrigger<'a, S> {
    pub fn select<E, T>(&mut self, event: E) -> impl Iterator<Item = &T::Wrapper>
    where
        E: FnOnce(&S) -> &Event<T>,
        T: EventType,
    {
        event(&self.0).subscribers.values()
    }
}
