use super::view::get_window;
use crate::ui::DesktopWindow;
use objc2::msg_send;
use std::ops::Deref;
use wae::Blocker;

/// Encapsulates a modal window and its parent.
///
/// This struct forces the modal window to be dropped before its parent.
pub struct Modal<'a, W, P>
where
    W: DesktopWindow,
    P: DesktopWindow,
{
    window: W,
    parent: &'a P,
    #[allow(dead_code)]
    blocker: Blocker<'a, P>,
}

impl<'a, W, P> Modal<'a, W, P>
where
    W: DesktopWindow,
    P: DesktopWindow,
{
    pub(super) fn new(window: W, parent: &'a P) -> Self {
        Self {
            window,
            parent,
            blocker: wae::block(parent),
        }
    }
}

impl<'a, W, P> Drop for Modal<'a, W, P>
where
    W: DesktopWindow,
    P: DesktopWindow,
{
    fn drop(&mut self) {
        let w = get_window(self.window.handle());
        let p = get_window(self.parent.handle());
        let _: () = unsafe { msg_send![p, endSheet:w] };
    }
}

impl<'a, W, P> Deref for Modal<'a, W, P>
where
    W: DesktopWindow,
    P: DesktopWindow,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
