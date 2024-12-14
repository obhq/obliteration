use std::collections::hash_map::OccupiedEntry;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// List of pending tasks.
#[derive(Default)]
pub struct TaskList {
    list: HashMap<u64, Pin<Box<dyn Future<Output = ()>>>>,
    next: u64,
}

impl TaskList {
    pub fn insert(&mut self, f: Pin<Box<dyn Future<Output = ()>>>) -> u64 {
        let id = self.next;

        assert!(self.list.insert(id, f).is_none());
        self.next = self.next.checked_add(1).unwrap();

        id
    }

    pub fn get(
        &mut self,
        id: u64,
    ) -> Option<OccupiedEntry<u64, Pin<Box<dyn Future<Output = ()>>>>> {
        use std::collections::hash_map::Entry;

        match self.list.entry(id) {
            Entry::Occupied(e) => Some(e),
            Entry::Vacant(_) => None,
        }
    }
}
