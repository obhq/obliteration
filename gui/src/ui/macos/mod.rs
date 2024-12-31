use super::PlatformExt;
use crate::rt::RuntimeWindow;
use objc::runtime::Object;
use objc::{msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use thiserror::Error;

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        // Get NSView.
        let win = self.window().window_handle();
        let win = win.window_handle().unwrap();
        let win = match win.as_ref() {
            RawWindowHandle::AppKit(v) => v.ns_view.as_ptr().cast::<Object>(),
            _ => unreachable!(),
        };

        // Get NSWindow and call center() method.
        let win: *mut Object = unsafe { msg_send![win, window] };
        let _: () = unsafe { msg_send![win, center] };

        Ok(())
    }

    fn set_modal<P>(&self, parent: &P) -> Result<(), PlatformError>
    where
        P: RuntimeWindow + ?Sized,
    {
        todo!()
    }
}

/// macOS-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
