use super::Event;
use std::sync::Arc;
use std::task::Wake;
use winit::event_loop::EventLoopProxy;

/// Implementation of [`Wake`].
pub struct Waker {
    el: EventLoopProxy<Event>,
    task: u64,
}

impl Waker {
    pub fn new(el: EventLoopProxy<Event>, task: u64) -> Self {
        Self { el, task }
    }
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        drop(self.el.send_event(Event::TaskReady(self.task)));
    }
}
