pub use self::dialogs::*;

use self::modal::Modal;
use super::DesktopWindow;
use crate::rt::WinitWindow;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::io::Error;
use std::mem::zeroed;
use thiserror::Error;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowRect, SetWindowPos, HWND_TOP, SM_CXSCREEN, SM_CYSCREEN, SWP_NOSIZE,
    SWP_NOZORDER,
};

mod dialogs;
mod modal;

impl<T: WinitWindow> DesktopWindow for T {
    type Modal<'a, P>
        = Modal<'a, Self, P>
    where
        P: WinitWindow + 'a;

    fn set_center(&self) -> Result<(), PlatformError> {
        // Get HWND.
        let win = self.handle();
        let win = win.window_handle().unwrap();
        let RawWindowHandle::Win32(win) = win.as_ref() else {
            unreachable!();
        };

        // Get window rectangle.
        let win = win.hwnd.get() as HWND;
        let mut rect = unsafe { zeroed() };
        let ret = unsafe { GetWindowRect(win, &mut rect) };

        if ret == 0 {
            return Err(PlatformError::GetWindowRect(Error::last_os_error()));
        }

        // TODO: Get width of the screen where the window belong to.
        let sw = unsafe { GetSystemMetrics(SM_CXSCREEN) };

        if sw == 0 {
            return Err(PlatformError::GetScreenWidth(Error::last_os_error()));
        }

        // TODO: Get height of the screen where the window belong to.
        let sh = unsafe { GetSystemMetrics(SM_CYSCREEN) };

        if sh == 0 {
            return Err(PlatformError::GetScreenHeight(Error::last_os_error()));
        }

        // TODO: Make this monitor aware.
        let ww = rect.right - rect.left;
        let wh = rect.bottom - rect.top;
        let x = (sw - ww) / 2;
        let y = (sh - wh) / 2;
        let ret = unsafe { SetWindowPos(win, HWND_TOP, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER) };

        if ret == 0 {
            return Err(PlatformError::SetWindowPos(Error::last_os_error()));
        }

        Ok(())
    }

    fn set_modal<P>(self, parent: &P) -> Result<Modal<Self, P>, PlatformError>
    where
        P: WinitWindow,
        Self: Sized,
    {
        todo!()
    }
}

/// Windows-specific error for [`DesktopWindow`].
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("couldn't get window rectangle")]
    GetWindowRect(#[source] Error),

    #[error("couldn't get screen width")]
    GetScreenWidth(#[source] Error),

    #[error("couldn't get screen height")]
    GetScreenHeight(#[source] Error),

    #[error("couldn't set window position")]
    SetWindowPos(#[source] Error),
}
