use crate::rt::{block, Blocker, WinitWindow};
use std::ops::Deref;
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1::XdgDialogV1;

/// Encapsulates a modal window and its parent.
///
/// This struct forces the modal window to be dropped before its parent.
pub struct Modal<'a, W, P: WinitWindow> {
    window: W,
    wayland: Option<XdgDialogV1>,
    #[allow(dead_code)]
    blocker: Blocker<'a, P>,
}

impl<'a, W, P: WinitWindow> Modal<'a, W, P> {
    pub fn new(window: W, parent: &'a P, wayland: Option<XdgDialogV1>) -> Self {
        Self {
            window,
            wayland,
            blocker: block(parent),
        }
    }
}

impl<'a, W, P: WinitWindow> Drop for Modal<'a, W, P> {
    fn drop(&mut self) {
        if let Some(v) = self.wayland.take() {
            v.destroy();
        }
    }
}

impl<'a, W, P: WinitWindow> Deref for Modal<'a, W, P> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
