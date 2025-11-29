use super::Channel;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// Provides a method to receive values from the channel created with [new](super::new()).
pub struct Receiver<T>(Arc<Channel<T>>);

impl<T> Receiver<T> {
    pub(super) fn new(chan: Arc<Channel<T>>) -> Self {
        Self(chan)
    }

    pub fn recv(&mut self) -> impl Future<Output = Option<T>> + Unpin + '_ {
        Recv {
            chan: &self.0,
            waiting: false,
        }
    }
}

/// Implementation of [`Future`] for [`Receiver::recv()`].
struct Recv<'a, T> {
    chan: &'a Channel<T>,
    waiting: bool,
}

impl<'a, T> Drop for Recv<'a, T> {
    fn drop(&mut self) {
        if self.waiting {
            self.chan.queue.lock().unwrap().waiter = None;
        }
    }
}

impl<'a, T> Future for Recv<'a, T> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Get item if available.
        let mut q = self.chan.queue.lock().unwrap();

        if let Some(v) = q.items.pop_front() {
            // The future may poll without a wakeup from a waker.
            q.waiter = None;

            self.waiting = false;
            self.chan.cv.notify_one();

            return Poll::Ready(Some(v));
        } else if q.senders == 0 {
            // We don't need to notify the condition variable here since no other sender waiting for
            // it.
            q.waiter = None;

            self.waiting = false;

            return Poll::Ready(None);
        }

        // Store waker.
        self.waiting = true;

        q.waiter = Some(cx.waker().clone());

        Poll::Pending
    }
}
