use std::fmt::{Display, Formatter};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn error_free(err: *mut Error) {
    unsafe { Box::from_raw(err) };
}

#[no_mangle]
pub extern "C" fn error_message(err: &Error) -> *const c_char {
    util::str::to_c(&err.to_string())
}

/// Represents an error to return to C world. Usually this will be using on any functions that
/// exposed to C world that need to return errors.
pub struct Error {
    causes: Vec<String>,
}

impl Error {
    pub fn new(src: &dyn std::error::Error) -> *mut Error {
        // Extract error messages from the source.
        let mut causes = vec![src.to_string()];
        let mut child = src.source();

        while let Some(e) = child {
            causes.push(e.to_string());
            child = e.source();
        }

        // Create instance.
        let error = Box::new(Self { causes });

        Box::into_raw(error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_str(&self.causes.join(" -> "))
    }
}
