use super::{DebugClient, DebugServer};
use crate::error::RustError;
use std::ffi::{c_char, CStr};
use std::ptr::null_mut;

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
    match DebugServer::new(addr) {
        Ok(server) => Box::into_raw(Box::new(server)),
        Err(e) => {
            *err = RustError::wrap(e).into_c();
            null_mut()
        }
    }
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
) -> *mut DebugClient {
    match (*s).accept() {
        Ok(client) => Box::into_raw(Box::new(client)),
        Err(e) => {
            *err = RustError::wrap(e).into_c();
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn debug_client_free(d: *mut DebugClient) {
    drop(Box::from_raw(d));
}
