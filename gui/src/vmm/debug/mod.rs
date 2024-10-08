// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

use gdbstub::conn::Connection;
use std::ffi::{c_int, c_void};

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

/// Encapsulates a C++ function on Qt side to provide [`Connection`] implementation.
pub struct Client {
    fp: unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut c_int) -> bool,
    cx: *mut c_void,
}

impl Client {
    pub fn new(
        fp: unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut c_int) -> bool,
        cx: *mut c_void,
    ) -> Self {
        Self { fp, cx }
    }
}

impl Connection for Client {
    type Error = c_int;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.write_all(std::slice::from_ref(&byte))
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        let mut e = 0;

        if unsafe { (self.fp)(self.cx, buf.as_ptr(), buf.len(), &mut e) } {
            Ok(())
        } else {
            Err(e)
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
