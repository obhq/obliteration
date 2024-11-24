// SPDX-License-Identifier: MIT OR Apache-2.0
use std::ffi::{c_char, c_void};

mod debug;
mod error;
mod graphics;
mod hv;
mod profile;
mod string;
mod system;
mod vmm;

#[cfg(feature = "qt")]
#[no_mangle]
pub unsafe extern "C-unwind" fn set_panic_hook(
    cx: *mut c_void,
    hook: unsafe extern "C-unwind" fn(*const c_char, usize, u32, *const c_char, usize, *mut c_void),
) {
    let cx = cx as usize;

    std::panic::set_hook(Box::new(move |info| {
        // Get location.
        let loc = info.location().unwrap();
        let file = loc.file();
        let line = loc.line();

        // Get message.
        //TODO: use payload_as_str() when https://github.com/rust-lang/rust/issues/125175 is stable.
        let msg = if let Some(&p) = info.payload().downcast_ref::<&str>() {
            p
        } else if let Some(p) = info.payload().downcast_ref::<String>() {
            p
        } else {
            "unknown panic payload"
        };

        // Invoke Qt.
        hook(
            file.as_ptr().cast(),
            file.len(),
            line,
            msg.as_ptr().cast(),
            msg.len(),
            cx as _,
        );
    }));
}
