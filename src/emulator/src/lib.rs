use self::emulator::Emulator;
use libc::{c_char, c_int};
use std::ptr::null_mut;

mod emulator;
mod pkg;

#[no_mangle]
pub extern "C" fn emulator_init(error: *mut *mut c_char) -> *mut Emulator {
    // Initialize SDL.
    let sdl = match sdl2::init() {
        Ok(v) => v,
        Err(v) => {
            set_error(&v, error);
            return null_mut();
        }
    };

    // Construct instance.
    let e = Box::new(Emulator::new(sdl));

    Box::into_raw(e)
}

#[no_mangle]
pub extern "C" fn emulator_term(e: *mut Emulator) {
    unsafe { Box::from_raw(e) };
}

#[no_mangle]
pub extern "C" fn emulator_start<'e, 'c>(
    _: &'e mut Emulator,
    _: &'c EmulatorConfig,
) -> *mut c_char {
    null_mut()
}

#[no_mangle]
pub extern "C" fn emulator_running<'e>(_: &'e mut Emulator) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn emulator_pkg_open<'e>(
    _: &'e mut Emulator,
    file: *const c_char,
    error: *mut *mut c_char,
) -> *mut pkg::PkgFile {
    let path = to_str(file);
    let pkg = match pkg::PkgFile::open(path) {
        Ok(v) => Box::new(v),
        Err(e) => {
            set_error(&e.to_string(), error);
            return null_mut();
        }
    };

    Box::into_raw(pkg)
}

#[no_mangle]
pub extern "C" fn emulator_pkg_close(pkg: *mut pkg::PkgFile) {
    unsafe { Box::from_raw(pkg) };
}

#[repr(C)]
pub struct EmulatorConfig {}

// This function assume ptr is a valid UTF-8 C string.
fn to_str<'a>(ptr: *const c_char) -> &'a str {
    let len = unsafe { libc::strlen(ptr) };
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };

    unsafe { std::str::from_utf8_unchecked(slice) }
}

fn set_error(msg: &str, dst: *mut *mut c_char) {
    let buf = unsafe { libc::malloc(msg.len() + 1) } as *mut c_char;

    if buf.is_null() {
        panic!("Out of memory");
    }

    unsafe { buf.copy_from_nonoverlapping(msg.as_ptr() as _, msg.len()) };
    unsafe { *buf.offset(msg.len() as _) = 0 };

    unsafe { *dst = buf };
}
