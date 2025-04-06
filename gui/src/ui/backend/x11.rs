use super::BackendError;
use raw_window_handle::{XcbDisplayHandle, XlibDisplayHandle};
use std::ptr::NonNull;
use xcb::x::InternAtom;

pub enum X11 {
    Xlib(Xlib),
    Xcb(Xcb),
}

pub struct Xlib {
    display: NonNull<x11::xlib::Display>,
    atoms: [x11::xlib::Atom; 5],
}

impl Xlib {
    pub unsafe fn new(handle: XlibDisplayHandle) -> Result<Self, BackendError> {
        let display = handle.display.unwrap().cast();

        let atom_names = [
            c"_NET_WM_WINDOW_TYPE".as_ptr(),
            c"_NET_WM_WINDOW_TYPE_DIALOG".as_ptr(),
            c"WM_TRANSIENT_FOR".as_ptr(),
            c"_NET_WM_STATE_MODAL".as_ptr(),
            c"_NET_WM_STATE".as_ptr(),
        ];

        let mut atoms = [0; 5];

        let ret = unsafe {
            x11::xlib::XInternAtoms(
                display.as_ptr(),
                atom_names.as_ptr() as _,
                atom_names.len() as i32,
                true as _,
                atoms.as_mut_ptr(),
            )
        };

        match ret {
            0 => Err(BackendError::XlibInternAtomsFailed),
            _ => Ok(Self { display, atoms }),
        }
    }

    pub fn display(&self) -> NonNull<x11::xlib::Display> {
        self.display
    }

    pub fn window_type_atom(&self) -> x11::xlib::Atom {
        self.atoms[0]
    }

    pub fn dialog_atom(&self) -> x11::xlib::Atom {
        self.atoms[1]
    }

    pub fn transient_for_atom(&self) -> x11::xlib::Atom {
        self.atoms[2]
    }

    pub fn wm_state_modal_atom(&self) -> x11::xlib::Atom {
        self.atoms[3]
    }

    pub fn wm_state_atom(&self) -> x11::xlib::Atom {
        self.atoms[4]
    }
}

pub struct Xcb {
    connection: xcb::Connection,

    window_type_atom: xcb::x::Atom,
    dialog_atom: xcb::x::Atom,
    transient_for_atom: xcb::x::Atom,
    wm_state_modal_atom: xcb::x::Atom,
    wm_state_atom: xcb::x::Atom,
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
        );

        let (window_type_atom, dialog_atom, transient_for_atom, wm_state_modal_atom, wm_state_atom) = (
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
        );

        Ok(Self {
            connection,
            window_type_atom,
            dialog_atom,
            transient_for_atom,
            wm_state_modal_atom,
            wm_state_atom,
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

    pub fn wm_state_modal_atom(&self) -> xcb::x::Atom {
        self.wm_state_modal_atom
    }

    pub fn wm_state_atom(&self) -> xcb::x::Atom {
        self.wm_state_atom
    }
}
