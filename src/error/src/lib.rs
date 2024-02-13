use std::ffi::CString;
use std::os::raw::c_char;

/// # Safety
/// `err` must be come from [`Error::new()`].
#[no_mangle]
pub unsafe extern "C" fn error_free(err: *mut Error) {
    drop(Box::from_raw(err));
}

#[no_mangle]
pub extern "C" fn error_message(err: &Error) -> *const c_char {
    err.message.as_ptr()
}

/// Represents an error to return to C world. Usually this will be using on any functions that
/// exposed to C world that need to return errors.
pub struct Error {
    message: CString,
}

impl Error {
    pub fn new<E: std::error::Error>(src: E) -> *mut Error {
        let mut causes = vec![src.to_string()];
        let mut child = src.source();

        while let Some(e) = child {
            causes.push(e.to_string());
            child = e.source();
        }

        Box::into_raw(Box::new(Self {
            message: CString::new(causes.join(" -> ")).unwrap(),
        }))
    }
}
