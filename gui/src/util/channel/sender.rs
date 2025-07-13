use super::Channel;
use std::num::NonZero;
use std::sync::Arc;

/// Provides a method to send values to the channel created with [new](super::new()).
pub struct Sender<T> {
    chan: Arc<Channel<T>>,
    max: NonZero<usize>,
}

impl<T> Sender<T> {
    pub(super) fn new(chan: Arc<Channel<T>>, max: NonZero<usize>) -> Self {
        Self { chan, max }
    }

    pub fn send(&self, v: T) {
        // Wait for available room.
        let q = self.chan.queue.lock().unwrap();
        let mut q = self
            .chan
            .cv
            .wait_while(q, move |s| s.items.len() >= self.max.get())
            .unwrap();

        // Store the value and wake waiter.
        q.items.push_back(v);

        if let Some(w) = q.waiter.take() {
            w.wake();
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            chan: self.chan.clone(),
            max: self.max,
        }
    }
}
