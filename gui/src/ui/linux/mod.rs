use super::{PlatformExt, SlintBackend};
use crate::rt::{global, RuntimeWindow};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use thiserror::Error;

mod wayland;

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        let win = self.window().window_handle();
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

    fn set_modal<P>(&self, parent: &P) -> Result<(), PlatformError>
    where
        P: RuntimeWindow + ?Sized,
    {
        let win = self.window().window_handle();
        let back = global::<SlintBackend>().unwrap();

        if let Some(v) = back.wayland() {
            self::wayland::set_modal(v, &win, parent)
        } else {
            todo!()
        }
    }
}

/// Linux-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
