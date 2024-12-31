use super::Event;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Wake;
use winit::event_loop::EventLoopProxy;

/// List of pending tasks.
pub struct TaskList {
    el: EventLoopProxy<Event>,
    list: FxHashMap<u64, Task>,
    next: u64,
}

impl TaskList {
    pub fn new(el: EventLoopProxy<Event>) -> Self {
        Self {
            el,
            list: HashMap::default(),
            next: 0,
        }
    }

    pub fn create(&mut self, task: impl Future<Output = ()> + 'static) -> Task {
        let id = self.next;

        self.next = self.next.checked_add(1).unwrap(); // It should be impossible but just in case.

        Task {
            future: Box::pin(task),
            waker: Arc::new(Waker {
                el: self.el.clone(),
                task: id,
            }),
        }
    }

    pub fn insert(&mut self, task: Task) -> u64 {
        let id = task.waker.task;
        assert!(self.list.insert(id, task).is_none());
        id
    }

    pub fn remove(&mut self, id: u64) -> Option<Task> {
        self.list.remove(&id)
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }
}

/// Contains a [`Future`] and its waker for a pending task.
///
/// We need to use a single waker per [`Future`] to make [`std::task::Waker::will_wake()`] works.
pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Arc<Waker>,
}

impl Task {
    pub fn future_mut(&mut self) -> Pin<&mut dyn Future<Output = ()>> {
        self.future.as_mut()
    }

    pub fn waker(&self) -> &Arc<impl Wake + Send + Sync + 'static> {
        &self.waker
    }
}

/// Implementation of [`Wake`].
struct Waker {
    el: EventLoopProxy<Event>,
    task: u64,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        drop(self.el.send_event(Event::TaskReady(self.task)));
    }
}
