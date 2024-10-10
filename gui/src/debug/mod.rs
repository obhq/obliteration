// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::error::RustError;
use std::ffi::{c_char, c_void, CStr, CString};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

#[no_mangle]
pub unsafe extern "C" fn debug_server_start(
    cx: *mut c_void,
    addr: *const c_char,
    err: *mut *mut RustError,
    cb: unsafe extern "C" fn(&DebuggerAccept, *mut c_void),
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

    // Enable non-blocking mode so the GUI can cancel listening thread.
    if let Err(e) = sock.set_nonblocking(true) {
        *err = RustError::with_source("couldn't enable non-blocking mode", e).into_c();
        return null_mut();
    }

    // Get effective address to let the user know.
    let addr = match sock.local_addr() {
        Ok(v) => CString::new(v.to_string()).unwrap(),
        Err(e) => {
            *err = RustError::with_source("couldn't get server address", e).into_c();
            return null_mut();
        }
    };

    // Start thread to wait for a connection.
    let stop = Arc::new(AtomicBool::new(false));
    let listener = std::thread::spawn({
        let cx = cx as usize;
        let stop = stop.clone();

        move || {
            while !stop.load(Ordering::Relaxed) {
                let r = match sock.accept() {
                    Ok((sock, _)) => DebuggerAccept::Ok {
                        debugger: Box::into_raw(Box::new(Debugger {
                            sock,
                            buf: Vec::new(),
                            next: 0,
                        })),
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                    Err(e) => DebuggerAccept::Err {
                        reason: RustError::wrap(e).into_c(),
                    },
                };

                cb(&r, cx as _);
                break;
            }
        }
    });

    // Return server object.
    let s = DebugServer {
        addr,
        listener: Some(listener),
        stop,
    };

    Box::into_raw(s.into())
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
pub unsafe extern "C" fn debugger_free(d: *mut Debugger) {
    drop(Box::from_raw(d));
}

/// TCP listener to accept a debugger connection.
pub struct DebugServer {
    addr: CString,
    listener: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl Drop for DebugServer {
    fn drop(&mut self) {
        // There are a small chance for memory leak if the listener thread queue the even to Qt
        // event loop when are are here but it is okay since the only cases the Qt will miss that
        // event is when it is being shutdown.
        self.stop.store(true, Ordering::Relaxed);
        self.listener.take().unwrap().join().unwrap();
    }
}

/// Status of debugger connection acception.
#[allow(dead_code)]
#[repr(C)]
pub enum DebuggerAccept {
    Ok { debugger: *mut Debugger },
    Err { reason: *mut RustError },
}

/// Encapsulate a debugger connection.
pub struct Debugger {
    sock: TcpStream,
    buf: Vec<u8>,
    next: usize,
}

impl Debugger {
    pub fn read(&mut self) -> Result<u8, std::io::Error> {
        // Fill the buffer if needed.
        while self.next == self.buf.len() {
            use std::io::ErrorKind;

            // Clear previous data.
            self.buf.clear();
            self.next = 0;

            // Read.
            let mut buf = [0; 1024];
            let len = match self.sock.read(&mut buf) {
                Ok(v) => v,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };

            if len == 0 {
                return Err(std::io::Error::from(ErrorKind::UnexpectedEof));
            }

            self.buf.extend_from_slice(&buf[..len]);
        }

        // Get byte.
        let b = self.buf[self.next];

        self.next += 1;

        Ok(b)
    }
}

impl gdbstub::conn::Connection for Debugger {
    type Error = std::io::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        Write::write_all(&mut self.sock, std::slice::from_ref(&byte))
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        Write::write_all(&mut self.sock, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Write::flush(&mut self.sock)
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        self.sock.set_nodelay(true)
    }
}
