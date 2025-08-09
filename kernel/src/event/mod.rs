pub use self::ty::*;

use crate::lock::{Mutex, MutexGuard};
use alloc::collections::btree_map::BTreeMap;

mod ty;

/// Encapsulate a set of [`Event`].
///
/// Usually there are only one [`EventSet`] per subsystem. The purpose of this struct is to prevent
/// race condition during subscribing and triggering multiple events. In other words, this struct
/// provide atomicity for subscription to multiple events in the set.
pub struct EventSet<S>(Mutex<S>); // TODO: Change to RwLock.

impl<S> EventSet<S> {
    pub fn trigger(&self) -> EventTrigger<'_, S> {
        EventTrigger(self.0.lock())
    }
}

impl<S: Default> Default for EventSet<S> {
    fn default() -> Self {
        Self(Mutex::default())
    }
}

/// Implementation of `eventhandler_list` structure.
///
/// Our implementation is different from PS4 version to make it idomatic to Rust.
pub struct Event<T: EventType> {
    subscribers: BTreeMap<(u32, u64), T::Wrapper>, // el_entries
}

impl<T: EventType> Default for Event<T> {
    fn default() -> Self {
        Self {
            subscribers: BTreeMap::new(),
        }
    }
}

/// Struct to trigger one or more events.
///
/// It is guarantee that no other handler in the current set will get registered until this struct
/// has been dropped.
pub struct EventTrigger<'a, S>(MutexGuard<'a, S>);

impl<S> EventTrigger<'_, S> {
    pub fn select<E, T>(&mut self, event: E) -> impl Iterator<Item = &T::Wrapper>
    where
        E: FnOnce(&S) -> &Event<T>,
        T: EventType,
    {
        event(&self.0).subscribers.values()
    }
}
