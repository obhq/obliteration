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
        let back = wae::global::<SlintBackend>().unwrap();

        match back.protocol_specific() {
            Some(ProtocolSpecific::Wayland(_)) => {} // Wayland doesn't allow windows to position themselves.
            Some(ProtocolSpecific::X11(x11)) => unsafe {
                self::x11::set_center(&x11, self)?;
            },
            None => unimplemented!(),
        };

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

    #[error("couldn't get window geometry")]
    XcbGetGeometry(#[source] xcb::Error),

    #[error("couldn't center window")]
    XcbCenterWindow(#[source] xcb::ProtocolError),

    #[error("couldn't set window type: {0}")]
    XlibSetWindowType(NonZero<i32>),

    #[error("couldn't set window wm state: {0}")]
    XlibSetWmState(NonZero<i32>),

    #[error("couldn't get window attributes: {0}")]
    XlibGetWindowAttributes(NonZero<i32>),

    #[error("couldn't center window: {0}")]
    XlibCenterWindow(NonZero<i32>),
}
