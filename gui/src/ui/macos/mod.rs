use self::view::with_window;
use super::{Modal, PlatformExt, PlatformWindow};
use block::ConcreteBlock;
use objc::{msg_send, sel, sel_impl};
use std::ffi::c_long;
use std::ops::Deref;
use thiserror::Error;

mod view;

impl<T: PlatformWindow> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        with_window::<()>(self.handle(), |win| unsafe { msg_send![win, center] });

        Ok(())
    }

    fn set_modal<P>(self, parent: &P) -> Result<Modal<Self, P>, PlatformError>
    where
        P: PlatformWindow,
        Self: Sized,
    {
        // Setup completionHandler.
        let cb = ConcreteBlock::new(move |_: c_long| {}).copy();

        // Show the sheet.
        let win = self.handle();
        let win = with_window(win, |w| w);
        let _: () = with_window(parent.handle(), |w| unsafe {
            msg_send![w, beginSheet:win completionHandler:cb.deref()]
        });

        Ok(Modal::new(self, parent))
    }
}

/// macOS-specific error for [`PlatformExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
