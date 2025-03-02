pub use self::dialogs::*;

use self::modal::Modal;
use self::view::get_window;
use super::{DesktopExt, DesktopWindow};
use block2::RcBlock;
use objc2::msg_send;
use std::ffi::c_long;
use std::ops::Deref;
use thiserror::Error;

mod dialogs;
mod modal;
mod view;

impl<T: DesktopWindow> DesktopExt for T {
    type Modal<'a, P>
        = Modal<'a, Self, P>
    where
        P: DesktopWindow + 'a;

    fn set_center(&self) -> Result<(), PlatformError> {
        let win = get_window(self.handle());
        let _: () = unsafe { msg_send![win, center] };
        Ok(())
    }

    fn set_modal<P>(self, parent: &P) -> Result<Modal<Self, P>, PlatformError>
    where
        P: DesktopWindow,
        Self: Sized,
    {
        // Setup completionHandler.
        let cb = RcBlock::new(move |_: c_long| {});

        // Show the sheet.
        let w = get_window(self.handle());
        let p = get_window(parent.handle());
        let _: () = unsafe { msg_send![p, beginSheet:w, completionHandler:cb.deref()] };

        Ok(Modal::new(self, parent))
    }
}

/// macOS-specific error for [`DesktopExt`].
#[derive(Debug, Error)]
pub enum PlatformError {}
