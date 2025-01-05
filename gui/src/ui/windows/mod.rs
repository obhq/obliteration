pub use self::dialogs::*;

use super::{Modal, PlatformExt};
use crate::rt::WinitWindow;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::io::Error;
use thiserror::Error;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowRect, SetWindowPos, HWND_TOP, SM_CXSCREEN, SM_CYSCREEN, SWP_NOSIZE,
    SWP_NOZORDER,
};

mod dialogs;

impl<T: WinitWindow> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        // Get HWND.
        let win = self.handle();
        let win = win.window_handle().unwrap();
        let RawWindowHandle::Win32(win) = win.as_ref() else {
            unreachable!();
        };

        unsafe {
            let hwnd = win.hwnd.get() as HWND;
            let mut rect = std::mem::zeroed();

            let ret = GetWindowRect(hwnd, &mut rect);

            if ret == 0 {
                return Err(PlatformError::GetWindowRect(Error::last_os_error()));
            }

            let win_width = rect.right - rect.left;
            let win_height = rect.bottom - rect.top;

            let screen_width = GetSystemMetrics(SM_CXSCREEN);

            if screen_width == 0 {
                return Err(PlatformError::GetScreenWidth(Error::last_os_error()));
            }

            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            if screen_height == 0 {
                return Err(PlatformError::GetScreenHeight(Error::last_os_error()));
            }

            let ret = SetWindowPos(
                hwnd,
                HWND_TOP,
                (screen_width - win_width) / 2,
                (screen_height - win_height) / 2,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER,
            );

            if ret == 0 {
                return Err(PlatformError::SetWindowPos(Error::last_os_error()));
            }
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

/// Windows-specific error for [`PlatformExt`].
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
