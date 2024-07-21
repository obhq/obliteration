use std::error::Error;
use std::ffi::{c_char, CString};
use std::fmt::Write;

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
    pub fn with_source(msg: impl AsRef<str>, src: impl Error) -> *mut Self {
        let mut msg = format!("{} -> {}", msg.as_ref(), src);
        let mut src = src.source();

        while let Some(e) = src {
            write!(msg, " -> {e}").unwrap();
            src = e.source();
        }

        Box::into_raw(Self(CString::new(msg).unwrap()).into())
    }

    pub fn wrap(src: impl Error) -> *mut Self {
        let mut msg = src.to_string();
        let mut src = src.source();

        while let Some(e) = src {
            write!(msg, " -> {e}").unwrap();
            src = e.source();
        }

        Box::into_raw(Self(CString::new(msg).unwrap()).into())
    }
}
