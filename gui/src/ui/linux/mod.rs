pub use self::dialogs::*;

use self::modal::Modal;
use super::backend::ProtocolSpecific;
use super::{DesktopExt, DesktopWindow, SlintBackend};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::num::NonZero;
use thiserror::Error;

mod dialogs;
mod modal;
mod wayland;
mod x11;

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
        let back = wae::global::<SlintBackend>().unwrap();

        let wayland = match back.protocol_specific() {
            Some(ProtocolSpecific::Wayland(wayland)) => unsafe {
                self::wayland::set_modal(wayland, &self, parent).map(Some)?
            },
            Some(ProtocolSpecific::X11(x11)) => unsafe {
                self::x11::set_modal(&x11, &self, parent)?;

                None
            },
            None => None,
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

    #[error("couldn't set window type")]
    XcbSetWindowType(#[source] xcb::ProtocolError),

    #[error("couldn't set window wm state")]
    XcbSetWmState(#[source] xcb::ProtocolError),

    #[error("couldn't set window parent")]
    XcbSetParent(#[source] xcb::ProtocolError),

    #[error("couldn't set window type: {0}")]
    XlibSetWindowType(NonZero<i32>),

    #[error("couldn't set window wm state: {0}")]
    XlibSetWmState(NonZero<i32>),

    #[error("couldn't set window parent")]
    XlibSetParent(NonZero<i32>),
}
