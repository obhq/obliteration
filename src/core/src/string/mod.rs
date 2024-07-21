use libc::malloc;
use std::alloc::{handle_alloc_error, Layout};
use std::ffi::c_char;

pub(crate) fn strdup(s: impl AsRef<str>) -> *mut c_char {
    // Alloc a buffer.
    let s = s.as_ref();
    let l = s.len() + 1;
    let p = unsafe { malloc(l).cast::<u8>() };

    if p.is_null() {
        handle_alloc_error(Layout::array::<u8>(l).unwrap());
    }

    // Copy.
    unsafe { p.copy_from_nonoverlapping(s.as_ptr(), s.len()) };
    unsafe { p.add(s.len()).write(0) };

    p.cast()
}
