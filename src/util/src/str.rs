use libc::c_char;

/// The returned string valid as long as `c` is valid.
pub fn from_c_unchecked<'a>(c: *const c_char) -> &'a str {
    let len = unsafe { libc::strlen(c) };
    let slice = unsafe { std::slice::from_raw_parts(c as *const u8, len) };

    unsafe { std::str::from_utf8_unchecked(slice) }
}

/// Returns a copy of C string allocated using `malloc` so it can return to C world.
pub fn to_c(s: &str) -> *mut c_char {
    let c = unsafe { libc::malloc(s.len() + 1) } as *mut c_char;

    if c.is_null() {
        panic!("Out of memory");
    }

    unsafe { c.copy_from_nonoverlapping(s.as_ptr() as _, s.len()) };
    unsafe { *c.offset(s.len() as _) = 0 };

    c
}

/// Allocate a memory using `malloc` then copy string to it and return it via `r`.
pub fn set_c(r: *mut *mut c_char, s: &str) {
    unsafe { *r = to_c(s) };
}
