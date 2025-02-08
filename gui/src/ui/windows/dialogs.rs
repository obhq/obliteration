use super::PlatformError;
use crate::ui::{DesktopWindow, FileType};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::OsString;
use std::num::NonZero;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    COINIT_SPEED_OVER_MEMORY,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, FOS_NOCHANGEDIR, SIGDN_FILESYSPATH,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OPENFILENAMEW};
use windows_sys::Win32::UI::Shell::{
    SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
};

pub async fn open_file<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Result<Option<PathBuf>, PlatformError> {
    let parent = get_hwnd(parent);
    let title: Vec<u16> = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let browse = move || unsafe {
        // Setup FileOpenDialog.
        let browser: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).unwrap();
        let mut opts = browser.GetOptions().unwrap();
        let filter = match ty {
            FileType::Firmware => COMDLG_FILTERSPEC {
                pszName: w!("Firmware Dump"),
                pszSpec: w!("*.obf"),
            },
        };

        opts |= FOS_NOCHANGEDIR;

        browser.SetFileTypes(&[filter]).unwrap();
        browser.SetOptions(opts).unwrap();
        browser.SetTitle(PCWSTR(title.as_ptr())).unwrap();

        // Show FileOpenDialog.
        let item = match browser.Show(HWND(parent.get() as _)) {
            Ok(_) => browser.GetResult().unwrap(),
            Err(_) => return Ok(None),
        };

        // Get file path.
        let buf = item.GetDisplayName(SIGDN_FILESYSPATH).unwrap();
        let path = OsString::from_wide(buf.as_wide());

        CoTaskMemFree(buf.0 as _);

        Ok(Some(PathBuf::from(path)))
    };

    spawn_modal(browse).await
}

pub async fn open_dir<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
) -> Result<Option<PathBuf>, PlatformError> {
    let parent = get_hwnd(parent);
    let title: Vec<u16> = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let browse = move || unsafe {
        // TODO: Use IFileDialog instead.
        let mut bi: BROWSEINFOW = std::mem::zeroed();

        bi.hwndOwner = parent.get() as _;
        bi.lpszTitle = title.as_ptr();
        bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;

        // Show the browser.
        let pidl = SHBrowseForFolderW(&mut bi);

        if pidl.is_null() {
            return Ok(None);
        }

        // Get directory path.
        let mut buf = [0u16; 260];
        let r = SHGetPathFromIDListW(pidl, buf.as_mut_ptr());

        CoTaskMemFree(pidl as _);

        if r == 0 {
            return Ok(None);
        }

        // Construct PathBuf.
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let path = OsString::from_wide(&buf[..len]);

        Ok(Some(PathBuf::from(path)))
    };

    spawn_modal(browse).await
}

fn get_hwnd<T: DesktopWindow>(win: &T) -> NonZero<isize> {
    let win = win.handle();
    let win = win.window_handle().unwrap();
    let RawWindowHandle::Win32(win) = win.as_ref() else {
        unreachable!();
    };

    win.hwnd
}

async fn spawn_modal<R, F>(f: F) -> R
where
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();

    std::thread::spawn(move || unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_SPEED_OVER_MEMORY).unwrap();
        assert!(tx.send(f()).is_ok());
        CoUninitialize();
    });

    rx.await.unwrap()
}
