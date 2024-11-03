// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::client::*;

use crate::error::RustError;
use std::ffi::{c_char, CStr, CString};
use std::net::{TcpListener, ToSocketAddrs};
use std::ptr::null_mut;
use thiserror::Error;

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
    let debug_server = DebugServer::new(addr);

    match debug_server {
        Ok(v) => Box::into_raw(Box::new(v)),
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

/// TCP listener to accept a debugger connection.
pub struct DebugServer {
    addr: CString,
    sock: TcpListener,
}

impl DebugServer {
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self, StartDebugServerError> {
        let sock = TcpListener::bind(addr).map_err(StartDebugServerError::BindFailed)?;
        let addr = sock
            .local_addr()
            .map_err(StartDebugServerError::GetAddrFailed)?;

        Ok(Self {
            addr: CString::new(addr.to_string()).unwrap(),
            sock,
        })
    }

    pub fn accept(&self) -> std::io::Result<DebugClient> {
        let (sock, _) = self.sock.accept()?;

        Ok(DebugClient::new(sock))
    }
}

#[derive(Debug, Error)]
pub enum StartDebugServerError {
    #[error("couldn't bind to the specified address")]
    BindFailed(#[source] std::io::Error),

    #[error("couldn't get server address")]
    GetAddrFailed(#[source] std::io::Error),
}
