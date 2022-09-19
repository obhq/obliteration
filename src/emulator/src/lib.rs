use libc::{c_char, c_int};
use std::ptr::null_mut;

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
    let emu = Box::new(Emulator { sdl });

    Box::into_raw(emu)
}

#[no_mangle]
pub extern "C" fn emulator_term(e: *mut Emulator) {
    unsafe { Box::from_raw(e) };
}

#[no_mangle]
pub extern "C" fn emulator_start(_: *mut Emulator, _: *const EmulatorConfig) -> *mut c_char {
    null_mut()
}

#[no_mangle]
pub extern "C" fn emulator_running(_: *mut Emulator) -> c_int {
    0
}

// We don't need repr(C) due to the outside will treat it as opaque pointer.
pub struct Emulator {
    sdl: sdl2::Sdl,
}

#[repr(C)]
pub struct EmulatorConfig {}

fn set_error(msg: &str, dst: *mut *mut c_char) {
    let buf = unsafe { libc::malloc(msg.len() + 1) } as *mut c_char;

    if buf.is_null() {
        panic!("Out of memory");
    }

    unsafe { buf.copy_from_nonoverlapping(msg.as_ptr() as _, msg.len()) };
    unsafe { *buf.offset(msg.len() as _) = 0 };

    unsafe { *dst = buf };
}
