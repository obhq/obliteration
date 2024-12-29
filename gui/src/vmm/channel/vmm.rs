use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::num::NonZero;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::task::{Context, Poll, Waker};

/// Provides method to send and receive events from the VMM.
///
/// The main different from [`futures::channel::mpsc::channel()`] is our implementation will block
/// the sender when the buffer is full.
pub struct VmmStream<T> {
    max: NonZero<usize>,
    state: Mutex<State<T>>,
    cv: Condvar,
}

impl<T> VmmStream<T> {
    pub fn new(buffer: NonZero<usize>) -> Self {
        Self {
            max: buffer,
            state: Mutex::new(State {
                items: VecDeque::default(),
                wakers: BTreeMap::default(),
            }),
            cv: Condvar::default(),
        }
    }

    pub fn recv(&self) -> impl Future<Output = T> + '_ {
        Recv {
            stream: self,
            pending: None,
        }
    }

    pub fn send(&self, v: T) {
        // Wait for available room.
        let state = self.state.lock().unwrap();
        let mut state = self
            .cv
            .wait_while(state, |s| s.items.len() >= self.max.get())
            .unwrap();

        // Store the value and wake one task.
        state.items.push_back(v);

        if let Some((_, w)) = state.wakers.pop_first() {
            w.wake();
        }
    }
}

/// Implementation of [`Future`] to receive a value from [`VmmStream`].
struct Recv<'a, T> {
    stream: &'a VmmStream<T>,
    pending: Option<u64>,
}

impl<'a, T> Drop for Recv<'a, T> {
    fn drop(&mut self) {
        let id = match self.pending.take() {
            Some(v) => v,
            None => return,
        };

        self.stream.state.lock().unwrap().wakers.remove(&id);
    }
}

impl<'a, T> Future for Recv<'a, T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Get item if available.
        let mut state = self.stream.state.lock().unwrap();

        if let Some(v) = state.items.pop_front() {
            if let Some(id) = self.pending.take() {
                // The future may poll without a wakeup from a waker.
                state.wakers.remove(&id);
            }

            self.stream.cv.notify_one();
            return Poll::Ready(v);
        }

        // Store waker.
        let id = self
            .pending
            .get_or_insert_with(|| WAKER_ID.fetch_add(1, Ordering::Relaxed));

        state.wakers.insert(*id, cx.waker().clone());

        Poll::Pending
    }
}

/// State of [`VmmStream`].
struct State<T> {
    items: VecDeque<T>,
    wakers: BTreeMap<u64, Waker>,
}

static WAKER_ID: AtomicU64 = AtomicU64::new(0);
