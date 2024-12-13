use futures::channel::oneshot::{Receiver, Sender};
use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use winit::window::WindowId;

/// List of one-shot channel waiting for a window event.
#[derive(Default)]
pub struct WindowEvent<T>(Vec<(WindowId, Sender<T>)>);

impl<T: Clone> WindowEvent<T> {
    pub fn wait(&mut self, win: WindowId) -> impl Future<Output = T> {
        let (tx, rx) = futures::channel::oneshot::channel();

        self.0.push((win, tx));

        Wait(rx)
    }

    pub fn raise(&mut self, win: WindowId, data: T) {
        // TODO: https://github.com/rust-lang/rust/issues/43244
        let mut i = 0;

        while i < self.0.len() {
            let s = if self.0[i].0 == win {
                self.0.remove(i).1
            } else {
                i += 1;
                continue;
            };

            s.send(data.clone()).ok();
        }
    }
}

/// Result of [`WindowEvent::wait()`].
struct Wait<T>(Receiver<T>);

impl<T> Future for Wait<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let r = match self.0.poll_unpin(cx) {
            Poll::Ready(v) => v,
            Poll::Pending => return Poll::Pending,
        };

        // The future only driven when the event loop is running so this should never panic.
        Poll::Ready(r.unwrap())
    }
}
