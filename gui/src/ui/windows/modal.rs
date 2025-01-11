use crate::rt::{block, Blocker, WinitWindow};
use std::ops::Deref;

/// Encapsulates a modal window and its parent.
///
/// This struct forces the modal window to be dropped before its parent.
pub struct Modal<'a, W, P: WinitWindow> {
    window: W,
    #[allow(dead_code)]
    blocker: Blocker<'a, P>,
}

impl<'a, W, P: WinitWindow> Modal<'a, W, P> {
    pub(super) fn new(window: W, parent: &'a P) -> Self {
        Self {
            window,
            blocker: block(parent),
        }
    }
}

impl<'a, W, P: WinitWindow> Deref for Modal<'a, W, P> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
