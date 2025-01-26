pub use self::dialogs::*;

use self::modal::Modal;
use super::{DesktopExt, DesktopWindow, SlintBackend};
use crate::rt::global;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use thiserror::Error;

mod dialogs;
mod modal;
mod wayland;

impl<T: DesktopWindow> DesktopExt for T {
    type Modal<'a, P>
        = Modal<'a, Self, P>
    where
        P: DesktopWindow + 'a;

    fn set_center(&self) -> Result<(), PlatformError> {
        let win = self.handle();
        let win = win.window_handle().unwrap();

        match win.as_ref() {
            RawWindowHandle::Xlib(_) => todo!(),
            RawWindowHandle::Xcb(_) => todo!(),
            RawWindowHandle::Wayland(_) => (), // Wayland don't allow window to position itself.
            RawWindowHandle::Drm(_) | RawWindowHandle::Gbm(_) => unimplemented!(),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn set_modal<P>(self, parent: &P) -> Result<Modal<Self, P>, PlatformError>
    where
        P: DesktopWindow,
        Self: Sized,
    {
        let back = global::<SlintBackend>().unwrap();
        let wayland = if let Some(v) = back.wayland() {
            // SAFETY: The Modal struct we construct below force the parent to outlive the modal
            // window.
            unsafe { self::wayland::set_modal(v, &self, parent).map(Some)? }
        } else {
            todo!()
        };

        Ok(Modal::new(self, parent, wayland))
    }
}

/// Linux-specific error for [`DesktopExt`].
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("couldn't create xdg_dialog_v1")]
    CreateXdgDialogV1(#[source] wayland_client::DispatchError),

    #[error("couldn't set window modal")]
    SetModal(#[source] wayland_client::DispatchError),
}
