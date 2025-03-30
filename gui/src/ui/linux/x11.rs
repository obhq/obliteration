use super::PlatformError;
use crate::ui::DesktopWindow;
use crate::ui::backend::X11;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::num::NonZero;
use xcb::XidNew;

pub unsafe fn set_modal(
    x11: &X11,
    target: &impl DesktopWindow,
    parent: &impl DesktopWindow,
) -> Result<(), PlatformError> {
    let parent = parent.handle();
    let parent_handle = parent.window_handle().unwrap();

    let target = target.handle();
    let target_handle = target.window_handle().unwrap();

    match (x11, parent_handle.as_raw(), target_handle.as_raw()) {
        (X11::Xlib(xlib), RawWindowHandle::Xlib(parent), RawWindowHandle::Xlib(target)) => {
            let display = xlib.display();

            let ret = unsafe {
                x11::xlib::XChangeProperty(
                    display.as_ptr(),
                    target.window,
                    xlib.window_type_atom(),
                    x11::xlib::XA_ATOM,
                    32,
                    x11::xlib::PropModeReplace,
                    &xlib.dialog_atom() as *const _ as *const _,
                    1,
                )
            };

            if let Some(err) = NonZero::new(ret) {
                return Err(PlatformError::XlibSetWindowType(err));
            }

            let ret = unsafe {
                x11::xlib::XChangeProperty(
                    display.as_ptr(),
                    target.window,
                    xlib.wm_state_atom(),
                    x11::xlib::XA_ATOM,
                    32,
                    x11::xlib::PropModeAppend,
                    &xlib.wm_state_modal_atom() as *const _ as *const _,
                    1,
                )
            };

            if let Some(err) = NonZero::new(ret) {
                return Err(PlatformError::XlibSetWmState(err));
            }

            let ret = unsafe {
                x11::xlib::XChangeProperty(
                    display.as_ptr(),
                    target.window,
                    xlib.transient_for_atom(),
                    x11::xlib::XA_WINDOW,
                    32,
                    x11::xlib::PropModeReplace,
                    &parent.window as *const _ as *const _,
                    1,
                )
            };

            if let Some(err) = NonZero::new(ret) {
                return Err(PlatformError::XlibSetWindowType(err));
            }
        }
        (X11::Xcb(xcb), RawWindowHandle::Xcb(parent), RawWindowHandle::Xcb(target)) => {
            let connection = xcb.connection();

            let cookie = connection.send_request_checked(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window: unsafe { xcb::x::Window::new(target.window.get()) },
                property: xcb.window_type_atom(),
                r#type: xcb::x::ATOM_ATOM,
                data: &[xcb.dialog_atom()],
            });

            connection
                .check_request(cookie)
                .map_err(PlatformError::XcbSetWindowType)?;

            let cookie = connection.send_request_checked(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Append,
                window: unsafe { xcb::x::Window::new(target.window.get()) },
                property: xcb.wm_state_atom(),
                r#type: xcb::x::ATOM_ATOM,
                data: &[xcb.wm_state_modal_atom()],
            });

            connection
                .check_request(cookie)
                .map_err(PlatformError::XcbSetWmState)?;

            let cookie = connection.send_request_checked(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window: unsafe { xcb::x::Window::new(target.window.get()) },
                property: xcb.transient_for_atom(),
                r#type: xcb::x::ATOM_WINDOW,
                data: &[unsafe { xcb::x::Window::new(parent.window.get()) }],
            });

            connection
                .check_request(cookie)
                .map_err(PlatformError::XcbSetParent)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
