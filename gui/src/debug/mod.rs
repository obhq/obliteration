// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::client::*;

use crate::error::RustError;
use std::ffi::{c_char, CStr, CString};
use std::net::TcpListener;
use std::ptr::null_mut;

mod client;

#[no_mangle]
pub unsafe extern "C" fn debug_server_start(
    addr: *const c_char,
    err: *mut *mut RustError,
) -> *mut DebugServer {
    // Get address.
    let addr = match CStr::from_ptr(addr).to_str() {
        Ok(v) => v,
        Err(_) => {
            *err = RustError::new("the specified address is not UTF-8").into_c();
            return null_mut();
        }
    };

    // Start server.
    let sock = match TcpListener::bind(addr) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't bind to the specified address", e).into_c();
            return null_mut();
        }
    };

    // Get effective address to let the user know.
    let addr = match sock.local_addr() {
        Ok(v) => CString::new(v.to_string()).unwrap(),
        Err(e) => {
            *err = RustError::with_source("couldn't get server address", e).into_c();
            return null_mut();
        }
    };

    // Return server object.
    Box::into_raw(Box::new(DebugServer { addr, sock }))
}

#[no_mangle]
pub unsafe extern "C" fn debug_server_free(s: *mut DebugServer) {
    drop(Box::from_raw(s));
}

#[no_mangle]
pub unsafe extern "C" fn debug_server_addr(s: *mut DebugServer) -> *const c_char {
    (*s).addr.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn debug_server_socket(s: *mut DebugServer) -> isize {
    #[cfg(unix)]
    return std::os::fd::AsRawFd::as_raw_fd(&(*s).sock) as _;

    #[cfg(windows)]
    return std::os::windows::io::AsRawSocket::as_raw_socket(&(*s).sock) as _;
}

#[no_mangle]
pub unsafe extern "C" fn debug_server_accept(
    s: *mut DebugServer,
    err: *mut *mut RustError,
) -> *mut Debugger {
    match (*s).sock.accept() {
        Ok((sock, _)) => Box::into_raw(Box::new(Debugger::new(sock))),
        Err(e) => {
            *err = RustError::wrap(e).into_c();
            null_mut()
        }
    }
}

/// TCP listener to accept a debugger connection.
pub struct DebugServer {
    addr: CString,
    sock: TcpListener,
}
