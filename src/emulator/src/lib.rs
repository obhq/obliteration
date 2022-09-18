use libc::{c_char, c_void};
use std::ptr::null_mut;

#[no_mangle]
pub extern "C" fn emulator_init(error: *mut *mut c_char) -> *mut c_void {
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

    Box::into_raw(emu) as _
}

#[no_mangle]
pub extern "C" fn emulator_term(inst: *mut c_void) {
    unsafe { Box::from_raw(inst as *mut Emulator) };
}

#[no_mangle]
pub extern "C" fn emulator_start(_: *const EmulatorConfig) -> *mut c_char {
    null_mut()
}

#[repr(C)]
pub struct EmulatorConfig {}

struct Emulator {
    sdl: sdl2::Sdl,
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
