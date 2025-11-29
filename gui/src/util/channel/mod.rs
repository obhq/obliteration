pub use self::receiver::*;
pub use self::sender::*;

use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::{Arc, Condvar, Mutex};
use std::task::Waker;

mod receiver;
mod sender;

/// Create a new channel.
///
/// The main different from [`futures::channel::mpsc::channel()`] is our implementation will block
/// the sender when the buffer is full.
pub fn new<T>(buffer: NonZero<usize>) -> (Sender<T>, Receiver<T>) {
    let chan = Arc::new(Channel::default());
    let sender = Sender::new(chan.clone(), buffer);
    let receiver = Receiver::new(chan);

    (sender, receiver)
}

/// State shared with [`Sender`] and [`Receiver`].
struct Channel<T> {
    queue: Mutex<Queue<T>>,
    cv: Condvar,
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(Queue {
                items: VecDeque::new(),
                waiter: None,
                senders: 1,
            }),
            cv: Condvar::new(),
        }
    }
}

/// Pending items in [Channel].
struct Queue<T> {
    items: VecDeque<T>,
    waiter: Option<Waker>,
    senders: usize,
}
