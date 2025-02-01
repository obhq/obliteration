use crate::ui::{DesktopWindow, FileType};
use block::ConcreteBlock;
use core_foundation::array::CFArrayGetValueAtIndex;
use core_foundation::base::TCFType;
use core_foundation::url::CFURL;
use futures::StreamExt;
use objc::runtime::{Object, NO, YES};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_long;
use std::ops::Deref;
use std::path::PathBuf;

#[allow(non_upper_case_globals)]
const NSModalResponseOK: c_long = 1;

pub async fn open_file<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Option<PathBuf> {
    todo!();
}

pub async fn open_dir<T: DesktopWindow>(parent: &T, title: impl AsRef<str>) -> Option<PathBuf> {
    // Get NSView of the parent window.
    let parent = parent.handle();
    let parent = parent.window_handle().unwrap();
    let parent = match parent.as_ref() {
        RawWindowHandle::AppKit(v) => v.ns_view.as_ptr() as *mut Object,
        _ => unreachable!(),
    };

    // Create NSOpenPanel.
    let panel: *mut Object = unsafe { msg_send![class!(NSOpenPanel), openPanel] };

    let _: () = unsafe { msg_send![panel, setCanChooseFiles:NO] };
    let _: () = unsafe { msg_send![panel, setCanChooseDirectories:YES] };

    // Setup handler.
    let (tx, mut rx) = futures::channel::mpsc::unbounded();
    let cb = ConcreteBlock::new(move |result: c_long| unsafe {
        if result != NSModalResponseOK {
            tx.unbounded_send(None).unwrap();
            return;
        }

        // Get selected URL.
        let url = CFArrayGetValueAtIndex(msg_send![panel, URLs], 0);
        let url: CFURL = CFURL::wrap_under_get_rule(url.cast());

        tx.unbounded_send(Some(url.to_path().unwrap())).unwrap();
    });

    // Show NSOpenPanel. It seems like beginSheetModalForWindow will take an onwership of the panel.
    let parent: *mut Object = unsafe { msg_send![parent, window] };
    let cb = cb.copy();

    let _: () =
        unsafe { msg_send![panel, beginSheetModalForWindow:parent completionHandler:cb.deref()] };

    // The beginSheetModalForWindow will return immediately so we need a channel here.
    rx.next().await.flatten()
}
