use crate::ui::{DesktopWindow, FileType};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::OsString;
use std::future::Future;
use std::num::NonZero;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::core::w;
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
) -> Option<PathBuf> {
    let parent = get_hwnd(parent);
    let title: Vec<u16> = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let browse = move || unsafe {
        // Setup CLSID_FileOpenDialog.
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
        browser.SetTitle(title.as_ptr()).unwrap();

        // Show CLSID_FileOpenDialog.
        let item = match browser.Show(HWND(parent.get() as _)) {
            Ok(_) => browser.GetResult().unwrap(),
            Err(_) => return None,
        };

        // Get file path.
        let buf = item.GetDisplayName(SIGDN_FILESYSPATH).unwrap();
        let path = {
            let mut len = 0;

            while *buf.add(len) != 0 {
                len += 1;
            }

            OsString::from_wide(std::slice::from_raw_parts(buf, len))
        };

        CoTaskMemFree(buf as _);

        Some(PathBuf::from(path))
    };

    spawn_modal(browse).await
}

pub async fn open_dir<T: DesktopWindow>(parent: &T, title: impl AsRef<str>) -> Option<PathBuf> {
    let parent = get_hwnd(parent);
    let title: Vec<u16> = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let browse = move || unsafe {
        let mut bi: BROWSEINFOW = std::mem::zeroed();

        bi.hwndOwner = parent.get() as _;
        bi.lpszTitle = title.as_ptr();
        bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;

        let pidl = SHBrowseForFolderW(&mut bi);
        if !pidl.is_null() {
            let mut buffer = [0u16; 260];
            if SHGetPathFromIDListW(pidl, buffer.as_mut_ptr()) != 0 {
                let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
                let path_str = String::from_utf16_lossy(&buffer[..len]);
                let dir_path = PathBuf::from(path_str);

                CoTaskMemFree(pidl as _);

                Some(dir_path)
            } else {
                CoTaskMemFree(pidl as _);
                None
            }
        } else {
            None
        }
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
