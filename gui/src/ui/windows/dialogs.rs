use crate::rt::WinitWindow;
use crate::ui::FileType;
use futures::channel::oneshot;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::future::Future;
use std::num::NonZero;
use std::path::PathBuf;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OPENFILENAMEW};
use windows_sys::Win32::UI::Shell::{
    SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
};

fn get_hwnd<T: WinitWindow>(parent: &T) -> NonZero<isize> {
    let parent = parent.handle();
    let parent = parent.window_handle().unwrap();
    let RawWindowHandle::Win32(win) = parent.as_ref() else {
        unreachable!();
    };

    win.hwnd
}

fn spawn_dialog<F>(dialog_fn: F) -> impl Future<Output = Option<PathBuf>>
where
    F: Send + 'static + FnOnce() -> Option<PathBuf>,
{
    let (tx, rx) = oneshot::channel();

    std::thread::spawn(move || {
        let res = dialog_fn();
        tx.send(res).unwrap();
    });

    async move { rx.await.unwrap_or(None) }
}

pub async fn open_file<T: WinitWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Option<PathBuf> {
    let hwnd = get_hwnd(parent);

    let title_wide: Vec<u16> = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    spawn_dialog(move || unsafe {
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd.get() as _;

        ofn.lpstrTitle = title_wide.as_ptr();

        const MAX_PATH: usize = 260;
        let mut file_buffer = [0u16; MAX_PATH];
        ofn.lpstrFile = file_buffer.as_mut_ptr();
        ofn.nMaxFile = file_buffer.len() as u32;

        // Filter
        match ty {
            FileType::Firmware => {
                let filter_str = "Firmware Dump\0*.obf\0\0"
                    .encode_utf16()
                    .collect::<Vec<u16>>();
                ofn.lpstrFilter = filter_str.as_ptr();
            }
        }

        let ret = GetOpenFileNameW(&mut ofn);
        if ret != 0 {
            let len = file_buffer
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(file_buffer.len());
            let path = String::from_utf16_lossy(&file_buffer[..len]);
            Some(PathBuf::from(path))
        } else {
            None
        }
    })
    .await
}

pub async fn open_dir<T: WinitWindow>(parent: &T, title: impl AsRef<str>) -> Option<PathBuf> {
    let hwnd = get_hwnd(parent);

    let title_wide: Vec<u16> = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    spawn_dialog(move || unsafe {
        let mut bi: BROWSEINFOW = std::mem::zeroed();
        bi.hwndOwner = hwnd.get() as _;

        bi.lpszTitle = title_wide.as_ptr();

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
    })
    .await
}
