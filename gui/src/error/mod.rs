// SPDX-License-Identifier: MIT OR Apache-2.0
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::{Display, Write};

#[cfg(feature = "qt")]
mod ffi;

/// Error object managed by Rust side.
pub struct RustError(Box<CStr>);

impl RustError {
    /// # Panics
    /// If `msg` contains NUL character.
    pub fn new(msg: impl Into<Vec<u8>>) -> Self {
        Self(CString::new(msg).unwrap().into_boxed_c_str())
    }

    pub fn with_source(msg: impl Display, src: impl Error) -> Self {
        let mut msg = format!("{} -> {}", msg, src);
        let mut src = src.source();

        while let Some(e) = src {
            write!(msg, " -> {e}").unwrap();
            src = e.source();
        }

        Self(CString::new(msg).unwrap().into_boxed_c_str())
    }

    pub fn wrap(src: impl Error) -> Self {
        let mut msg = src.to_string();
        let mut src = src.source();

        while let Some(e) = src {
            write!(msg, " -> {e}").unwrap();
            src = e.source();
        }

        Self(CString::new(msg).unwrap().into_boxed_c_str())
    }

    pub fn into_c(self) -> *mut Self {
        Box::into_raw(self.into())
    }
}
