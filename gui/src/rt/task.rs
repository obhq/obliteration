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
    pub fn insert(&mut self, id: Option<u64>, task: Pin<Box<dyn Future<Output = ()>>>) -> u64 {
        // Get ID.
        let id = match id {
            Some(v) => v,
            None => {
                let v = self.next;
                self.next = self.next.checked_add(1).unwrap();
                v
            }
        };

        assert!(self.list.insert(id, task).is_none());

        id
    }

    pub fn remove(&mut self, id: u64) -> Option<Pin<Box<dyn Future<Output = ()>>>> {
        self.list.remove(&id)
    }
}
