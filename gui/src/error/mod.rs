// SPDX-License-Identifier: MIT OR Apache-2.0
use std::error::Error;
use std::ffi::{c_char, CString};
use std::fmt::{Display, Write};

#[no_mangle]
pub unsafe extern "C" fn error_free(e: *mut RustError) {
    drop(Box::from_raw(e));
}

#[no_mangle]
pub unsafe extern "C" fn error_message(e: *const RustError) -> *const c_char {
    (*e).0.as_ptr()
}

/// Error object managed by Rust side.
pub struct RustError(CString);

impl RustError {
    /// # Panics
    /// If `msg` contains NUL character.
    pub fn new(msg: impl Into<Vec<u8>>) -> Self {
        Self(CString::new(msg).unwrap())
    }

    pub fn with_source(msg: impl Display, src: impl Error) -> Self {
        let mut msg = format!("{} -> {}", msg, src);
        let mut src = src.source();

        while let Some(e) = src {
            write!(msg, " -> {e}").unwrap();
            src = e.source();
        }

        Self(CString::new(msg).unwrap())
    }

    pub fn wrap(src: impl Error) -> Self {
        let mut msg = src.to_string();
        let mut src = src.source();

        while let Some(e) = src {
            write!(msg, " -> {e}").unwrap();
            src = e.source();
        }

        Self(CString::new(msg).unwrap())
    }

    pub fn into_c(self) -> *mut Self {
        Box::into_raw(self.into())
    }
}
