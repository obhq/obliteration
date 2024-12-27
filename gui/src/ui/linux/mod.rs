use super::PlatformExt;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use thiserror::Error;

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        let win = self.window().window_handle();
        let win = win.window_handle().unwrap();

        match win.as_ref() {
            RawWindowHandle::Xlib(xlib_window_handle) => todo!(),
            RawWindowHandle::Xcb(xcb_window_handle) => todo!(),
            RawWindowHandle::Wayland(_) => (), // Wayland don't allow window to position itself.
            RawWindowHandle::Drm(_) | RawWindowHandle::Gbm(_) => unimplemented!(),
            _ => unreachable!(),
        }

        Ok(())
    }
}

/// Linux-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
