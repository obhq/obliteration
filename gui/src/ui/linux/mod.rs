use super::{Modal, PlatformExt, PlatformWindow, SlintBackend};
use crate::rt::global;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use thiserror::Error;

mod wayland;

impl<T: PlatformWindow> PlatformExt for T {
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
        P: PlatformWindow,
        Self: Sized,
    {
        let win = self.handle();
        let back = global::<SlintBackend>().unwrap();

        if let Some(v) = back.wayland() {
            self::wayland::set_modal(v, win, parent.handle())?;
        } else {
            todo!()
        }

        Ok(Modal::new(self, parent))
    }
}

/// Linux-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
