use super::BackendError;
use raw_window_handle::{XcbDisplayHandle, XlibDisplayHandle};
use xcb::x::InternAtom;

pub enum X11 {
    Xlib(Xlib),
    Xcb(Xcb),
}

pub struct Xlib {
    handle: XlibDisplayHandle,
}

impl Xlib {
    pub unsafe fn new(handle: XlibDisplayHandle) -> Result<Self, BackendError> {
        todo!()
    }
}

pub struct Xcb {
    connection: xcb::Connection,

    window_type_atom: xcb::x::Atom,
    dialog_atom: xcb::x::Atom,
    transient_for_atom: xcb::x::Atom,
    modal_atom: xcb::x::Atom,
    wm_state_atom: xcb::x::Atom,
    wm_state_modal_atom: xcb::x::Atom,
}

impl Xcb {
    pub unsafe fn new(handle: XcbDisplayHandle) -> Result<Self, BackendError> {
        let connection = unsafe {
            xcb::Connection::from_raw_conn(handle.connection.unwrap().as_ptr() as *mut _)
        };

        let cookies = (
            connection.send_request(&InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_WINDOW_TYPE",
            }),
            connection.send_request(&InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_WINDOW_TYPE_DIALOG",
            }),
            connection.send_request(&InternAtom {
                only_if_exists: true,
                name: b"WM_TRANSIENT_FOR",
            }),
            connection.send_request(&InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MODAL",
            }),
            connection.send_request(&InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE",
            }),
            connection.send_request(&InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MODAL",
            }),
        );

        let (
            window_type_atom,
            dialog_atom,
            transient_for_atom,
            modal_atom,
            wm_state_atom,
            wm_state_modal_atom,
        ) = (
            connection
                .wait_for_reply(cookies.0)
                .map_err(BackendError::DispatchXcb)?
                .atom(),
            connection
                .wait_for_reply(cookies.1)
                .map_err(BackendError::DispatchXcb)?
                .atom(),
            connection
                .wait_for_reply(cookies.2)
                .map_err(BackendError::DispatchXcb)?
                .atom(),
            connection
                .wait_for_reply(cookies.3)
                .map_err(BackendError::DispatchXcb)?
                .atom(),
            connection
                .wait_for_reply(cookies.4)
                .map_err(BackendError::DispatchXcb)?
                .atom(),
            connection
                .wait_for_reply(cookies.5)
                .map_err(BackendError::DispatchXcb)?
                .atom(),
        );

        Ok(Self {
            connection,
            window_type_atom,
            dialog_atom,
            transient_for_atom,
            modal_atom,
            wm_state_atom,
            wm_state_modal_atom,
        })
    }

    pub fn connection(&self) -> &xcb::Connection {
        &self.connection
    }

    pub fn window_type_atom(&self) -> xcb::x::Atom {
        self.window_type_atom
    }

    pub fn dialog_atom(&self) -> xcb::x::Atom {
        self.dialog_atom
    }

    pub fn transient_for_atom(&self) -> xcb::x::Atom {
        self.transient_for_atom
    }

    pub fn modal_atom(&self) -> xcb::x::Atom {
        self.modal_atom
    }

    pub fn wm_state_atom(&self) -> xcb::x::Atom {
        self.wm_state_atom
    }

    pub fn wm_state_modal_atom(&self) -> xcb::x::Atom {
        self.wm_state_modal_atom
    }
}
