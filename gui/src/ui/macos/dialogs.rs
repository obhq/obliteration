use super::PlatformError;
use super::view::get_window;
use crate::ui::DesktopWindow;
use block2::RcBlock;
use futures::StreamExt;
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject};
use objc2::{class, msg_send};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;
use std::ffi::c_long;
use std::ops::Deref;
use std::path::PathBuf;

#[allow(non_upper_case_globals)]
const NSModalResponseOK: c_long = 1;

pub async fn open_file<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
    _: impl AsRef<str>,
    file_ext: impl AsRef<str>,
) -> Result<Option<PathBuf>, PlatformError> {
    let title = NSString::from_str(title.as_ref());
    let ty = NSString::from_str(file_ext.as_ref());
    let ty = unsafe { UTType::typeWithFilenameExtension(&ty).unwrap() };
    let panel: *mut NSObject = unsafe { msg_send![class!(NSOpenPanel), openPanel] };
    let types = NSArray::from_slice(&[ty.deref()]);

    let _: () = unsafe { msg_send![panel, setMessage:title.deref()] };
    let _: () = unsafe { msg_send![panel, setAllowedContentTypes:types.deref()] };

    open(parent, panel).await
}

pub async fn open_dir<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
) -> Result<Option<PathBuf>, PlatformError> {
    let title = NSString::from_str(title.as_ref());
    let panel: *mut NSObject = unsafe { msg_send![class!(NSOpenPanel), openPanel] };

    let _: () = unsafe { msg_send![panel, setCanChooseFiles:Bool::NO] };
    let _: () = unsafe { msg_send![panel, setCanChooseDirectories:Bool::YES] };
    let _: () = unsafe { msg_send![panel, setMessage:title.deref()] };

    open(parent, panel).await
}

async fn open<T: DesktopWindow>(
    parent: &T,
    panel: *mut NSObject,
) -> Result<Option<PathBuf>, PlatformError> {
    // Setup handler.
    let (tx, mut rx) = futures::channel::mpsc::unbounded();
    let cb = RcBlock::new(move |result: c_long| unsafe {
        if result != NSModalResponseOK {
            tx.unbounded_send(None).unwrap();
            return;
        }

        // Get selected URL.
        let urls: Retained<NSArray<NSURL>> = msg_send![panel, URLs];
        let url = urls.objectAtIndex(0);
        let path = url.path().unwrap();
        let path = path.to_string();

        tx.unbounded_send(Some(path.into())).unwrap();
    });

    // Show NSOpenPanel. It seems like beginSheetModalForWindow will take an onwership of the panel.
    let parent = get_window(parent.handle());

    let _: () =
        unsafe { msg_send![panel, beginSheetModalForWindow:parent, completionHandler:cb.deref()] };

    // The beginSheetModalForWindow will return immediately so we need a channel here.
    Ok(rx.next().await.flatten())
}
