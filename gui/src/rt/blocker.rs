use super::context::Context;
use super::WinitWindow;
use std::future::Future;
use std::marker::PhantomData;
use std::num::NonZero;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Poll;

/// RAII struct to unblock the window when dropped.
pub struct Blocker<'a, W: WinitWindow> {
    win: &'a W,
    phantom: PhantomData<Rc<()>>, // For !Send and !Sync.
}

impl<'a, W: WinitWindow> Blocker<'a, W> {
    pub(super) fn new(win: &'a W) -> Self {
        Self {
            win,
            phantom: PhantomData,
        }
    }
}

impl<'a, W: WinitWindow> Drop for Blocker<'a, W> {
    fn drop(&mut self) {
        use std::collections::hash_map::Entry;

        Context::with(|cx| {
            let Entry::Occupied(mut e) = cx.blocking.entry(self.win.id()) else {
                unreachable!();
            };

            match NonZero::new(e.get().get() - 1) {
                Some(v) => *e.get_mut() = v,
                None => drop(e.remove()),
            }
        })
    }
}

/// RAII struct to unblock the window when dropped.
pub(super) struct AsyncBlocker<W: WinitWindow, F> {
    win: W,
    task: F,
}

impl<W: WinitWindow, F> AsyncBlocker<W, F> {
    pub fn new(win: W, task: F) -> Self {
        Self { win, task }
    }
}

impl<W: WinitWindow, F> Drop for AsyncBlocker<W, F> {
    fn drop(&mut self) {
        use std::collections::hash_map::Entry;

        Context::with(|cx| {
            let Entry::Occupied(mut e) = cx.blocking.entry(self.win.id()) else {
                unreachable!();
            };

            match NonZero::new(e.get().get() - 1) {
                Some(v) => *e.get_mut() = v,
                None => drop(e.remove()),
            }
        })
    }
}

impl<W: WinitWindow, F: Future> Future for AsyncBlocker<W, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We did not move out the value from task.
        unsafe { self.map_unchecked_mut(|v| &mut v.task).poll(cx) }
    }
}
