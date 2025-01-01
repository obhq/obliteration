use super::PlatformExt;
use crate::rt::RuntimeWindow;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use thiserror::Error;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetSystemMetrics, GetWindowRect, SetWindowPos, HWND_TOP, SM_CXSCREEN, SM_CYSCREEN,
        SWP_NOSIZE, SWP_NOZORDER,
    },
};

impl<T: ComponentHandle> PlatformExt for T {
    fn set_center(&self) -> Result<(), PlatformError> {
        let win = self.window().window_handle();
        let raw_handle = win.window_handle().unwrap();

        let RawWindowHandle::Win32(h) = raw_handle.as_ref() else {
            unreachable!("Unsupported handle type on Windows");
        };

        unsafe {
            let hwnd = h.hwnd.get() as HWND;
            let mut rect = std::mem::zeroed();

            let ret = GetWindowRect(hwnd, &mut rect);

            if ret == 0 {
                return Err(PlatformError::GetWindowRectFailed(
                    std::io::Error::last_os_error(),
                ));
            }

            let win_width = rect.right - rect.left;
            let win_height = rect.bottom - rect.top;

            let screen_width = GetSystemMetrics(SM_CXSCREEN);

            if screen_width == 0 {
                return Err(PlatformError::GetScreenWidthFailed(
                    std::io::Error::last_os_error(),
                ));
            }

            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            if screen_height == 0 {
                return Err(PlatformError::GetScreenHeightFailed(
                    std::io::Error::last_os_error(),
                ));
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
                return Err(PlatformError::SetWindowPosFailed(
                    std::io::Error::last_os_error(),
                ));
            }
        }

        Ok(())
    }

    fn set_modal<P>(&self, parent: &P) -> Result<(), PlatformError>
    where
        P: RuntimeWindow + ?Sized,
    {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("failed to get window rect")]
    GetWindowRectFailed(#[source] std::io::Error),

    #[error("failed to get screen width")]
    GetScreenWidthFailed(#[source] std::io::Error),

    #[error("failed to get screen height")]
    GetScreenHeightFailed(#[source] std::io::Error),

    #[error("failed to set window position")]
    SetWindowPosFailed(#[source] std::io::Error),
}
