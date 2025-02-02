use objc2::msg_send;
use objc2::runtime::NSObject;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The returned `NSWindow` will be valid while `win` still alive.
pub fn get_window(win: impl HasWindowHandle) -> *mut NSObject {
    let win = get_view(win);

    unsafe { msg_send![win, window] }
}

/// The returned `NSView` will be valid while `win` still alive.
pub fn get_view(win: impl HasWindowHandle) -> *mut NSObject {
    let win = win.window_handle().unwrap();

    match win.as_ref() {
        RawWindowHandle::AppKit(v) => v.ns_view.as_ptr().cast(),
        _ => unreachable!(),
    }
}
