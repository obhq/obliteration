use std::ops::Deref;

/// Encapsulates a modal window and its parent.
///
/// This struct force the modal window to be dropped before its parent.
pub struct Modal<'a, W, P> {
    window: W,
    parent: &'a P,
}

impl<'a, W, P> Modal<'a, W, P> {
    pub(super) fn new(window: W, parent: &'a P) -> Self {
        Self { window, parent }
    }
}

impl<'a, W, P> Deref for Modal<'a, W, P> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
