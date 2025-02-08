use super::view::get_window;
use super::PlatformError;
use crate::ui::{DesktopWindow, FileType};
use block2::RcBlock;
use futures::StreamExt;
use objc2::ffi::{NO, YES};
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::NSObject;
use objc2::{class, msg_send, msg_send_id};
use objc2_foundation::{NSArray, NSString, NSURL};
use std::ffi::c_long;
use std::ops::Deref;
use std::path::PathBuf;

#[allow(non_upper_case_globals)]
const NSModalResponseOK: c_long = 1;

pub async fn open_file<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Result<Option<PathBuf>, PlatformError> {
    todo!();
}

pub async fn open_dir<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
) -> Result<Option<PathBuf>, PlatformError> {
    // Create NSOpenPanel.
    let title = NSString::from_str(title.as_ref());
    let panel: *mut NSObject = unsafe { msg_send![class!(NSOpenPanel), openPanel] };

    let _: () = unsafe { msg_send![panel, setCanChooseFiles:NO] };
    let _: () = unsafe { msg_send![panel, setCanChooseDirectories:YES] };
    let _: () = unsafe { msg_send![panel, setMessage:title.deref()] };

    // Setup handler.
    let (tx, mut rx) = futures::channel::mpsc::unbounded();
    let cb = RcBlock::new(move |result: c_long| unsafe {
        if result != NSModalResponseOK {
            tx.unbounded_send(None).unwrap();
            return;
        }

        // Get selected URL.
        let urls: Retained<NSArray<NSURL>> = msg_send_id![panel, URLs];
        let url = &urls[0];
        let path = url.path().unwrap();
        let path = autoreleasepool(move |p| path.as_str(p).to_owned());

        tx.unbounded_send(Some(path.into())).unwrap();
    });

    // Show NSOpenPanel. It seems like beginSheetModalForWindow will take an onwership of the panel.
    let parent = get_window(parent.handle());

    let _: () =
        unsafe { msg_send![panel, beginSheetModalForWindow:parent completionHandler:cb.deref()] };

    // The beginSheetModalForWindow will return immediately so we need a channel here.
    Ok(rx.next().await.flatten())
}
