use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Contains a value that will be available in the future.
///
/// Once the value is set it will always available. There is no way to unset the value.
///
/// This type requires async executor that has a single waker per [`Future`].
#[derive(Default)]
pub struct Signal<T> {
    value: OnceCell<T>,
    wakers: RefCell<HashMap<*const (), Waker>>,
}

impl<T> Signal<T> {
    pub fn set(&self, value: T) -> Result<(), T> {
        self.value.set(value)?;

        for (_, w) in self.wakers.borrow_mut().drain() {
            w.wake();
        }

        Ok(())
    }

    pub fn wait(&self) -> WaitForSignal<T> {
        WaitForSignal {
            signal: self,
            waker: None,
        }
    }
}

/// Implementation of [`Future`] to get the value from [`Signal`].
pub struct WaitForSignal<'a, T> {
    signal: &'a Signal<T>,
    waker: Option<Waker>,
}

impl<'a, T> Drop for WaitForSignal<'a, T> {
    fn drop(&mut self) {
        // We need a full Waker instead of its data so we don't accidentally remove a wrong waker if
        // it already been freed somehow.
        let w = match self.waker.take() {
            Some(v) => v,
            None => return,
        };

        // The waker may already removed from the list by Signal::set() when we are here.
        self.signal.wakers.borrow_mut().remove(&w.data());
    }
}

impl<'a, T> Future for WaitForSignal<'a, T> {
    type Output = &'a T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if value available.
        if let Some(v) = self.signal.value.get() {
            return Poll::Ready(v);
        }

        // Store waker. We requires async executor that has a single waker per future so we don't
        // need to store the latest waker.
        if self.waker.is_none() {
            let w = cx.waker().clone();

            assert!(self
                .signal
                .wakers
                .borrow_mut()
                .insert(w.data(), w.clone())
                .is_none());

            self.waker = Some(w);
        }

        Poll::Pending
    }
}
