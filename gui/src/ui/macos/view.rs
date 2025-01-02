use objc::runtime::Object;
use objc::{msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub fn with_window<T>(win: impl HasWindowHandle, f: impl FnOnce(*mut Object) -> T) -> T {
    // Get NSView.
    let win = win.window_handle().unwrap();
    let win = match win.as_ref() {
        RawWindowHandle::AppKit(v) => v.ns_view.as_ptr().cast::<Object>(),
        _ => unreachable!(),
    };

    // Get NSWindow.
    f(unsafe { msg_send![win, window] })
}
