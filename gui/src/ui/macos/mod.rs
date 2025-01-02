use self::view::with_window;
use super::PlatformExt;
use crate::rt::RuntimeWindow;
use block::ConcreteBlock;
use objc::{msg_send, sel, sel_impl};
use slint::ComponentHandle;
use std::ffi::c_long;
use std::ops::Deref;
use thiserror::Error;

mod view;

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        with_window::<()>(&self.window().window_handle(), |win| unsafe {
            msg_send![win, center]
        });

        Ok(())
    }

    fn set_modal<P>(&self, parent: &P) -> Result<(), PlatformError>
    where
        P: RuntimeWindow + ?Sized,
    {
        // Setup completionHandler.
        let cb = ConcreteBlock::new(move |_: c_long| {}).copy();

        // Show the sheet.
        let win = self.window().window_handle();
        let win = with_window(&win, |w| w);
        let _: () = with_window(parent, |w| unsafe {
            msg_send![w, beginSheet:win completionHandler:cb.deref()]
        });

        Ok(())
    }
}

/// macOS-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
