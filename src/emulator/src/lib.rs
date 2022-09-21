use context::Context;
use std::os::raw::{c_char, c_int};
use std::ptr::null_mut;

#[no_mangle]
pub extern "C" fn emulator_start(_: &mut Context, _: &EmulatorConfig) -> *mut c_char {
    null_mut()
}

#[no_mangle]
pub extern "C" fn emulator_running(_: &Context) -> c_int {
    0
}

#[repr(C)]
pub struct EmulatorConfig {}
