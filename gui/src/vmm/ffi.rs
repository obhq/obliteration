use super::{DebugResult, DispatchDebugResult, KernelStop, Vmm, VmmEvent, VmmScreen};
use crate::debug::DebugClient;
use crate::error::RustError;
use crate::profile::Profile;
use crate::screen::Screen;
use gdbstub::stub::state_machine::GdbStubStateMachine;
use std::ffi::{c_char, c_void, CStr};
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

#[no_mangle]
pub unsafe extern "C" fn vmm_start(
    kernel: *const c_char,
    screen: *const VmmScreen,
    profile: *const Profile,
    debugger: *mut DebugClient,
    event_handler: unsafe extern "C" fn(*const VmmEvent, *mut c_void),
    cx: *mut c_void,
    err: *mut *mut RustError,
) -> *mut Vmm {
    // Consume the debugger now to prevent memory leak in case of error.
    let debugger = if debugger.is_null() {
        None
    } else {
        Some(*Box::from_raw(debugger))
    };

    // Check if path UTF-8.
    let path = match CStr::from_ptr(kernel).to_str() {
        Ok(v) => v,
        Err(_) => {
            *err = RustError::new("path of the kernel is not UTF-8").into_c();
            return null_mut();
        }
    };

    let profile = unsafe { &*profile };
    let screen = unsafe { &*screen };

    let screen = match crate::screen::Default::from_screen(screen) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source("couldn't setup a screen", e).into_c();
            return null_mut();
        }
    };

    // Cast to make the closure Send + Sync.
    let cx_usize = cx as usize;

    match Vmm::new(path, screen, profile, debugger, move |event| {
        event_handler(&event, cx_usize as *mut c_void)
    }) {
        Ok(vmm) => Box::into_raw(Box::new(vmm)),
        Err(e) => {
            *err = RustError::wrap(e).into_c();
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_draw(vmm: *mut Vmm) -> *mut RustError {
    match (*vmm).screen.update() {
        Ok(_) => null_mut(),
        Err(e) => RustError::wrap(e).into_c(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_dispatch_debug(vmm: *mut Vmm, stop: *mut KernelStop) -> DebugResult {
    // Consume stop reason now to prevent memory leak.
    let vmm = &mut *vmm;
    let stop = if stop.is_null() {
        None
    } else {
        Some(Box::from_raw(stop).0)
    };

    match vmm.dispatch_debug(stop) {
        Ok(DispatchDebugResult::Ok) => DebugResult::Ok,
        Ok(DispatchDebugResult::Disconnected) => DebugResult::Disconnected,
        Err(e) => {
            let e = RustError::wrap(e).into_c();
            DebugResult::Error { reason: e }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_debug_socket(vmm: *mut Vmm) -> isize {
    let s = match &mut (*vmm).gdb {
        Some(v) => v,
        None => return -1,
    };

    match s {
        GdbStubStateMachine::Idle(s) => s.borrow_conn().socket() as _,
        GdbStubStateMachine::Running(s) => s.borrow_conn().socket() as _,
        GdbStubStateMachine::CtrlCInterrupt(s) => s.borrow_conn().socket() as _,
        GdbStubStateMachine::Disconnected(s) => s.borrow_conn().socket() as _,
    }
}

#[no_mangle]
pub unsafe extern "C" fn vmm_shutdown(vmm: *mut Vmm) {
    (*vmm).shutdown.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn vmm_shutting_down(vmm: *mut Vmm) -> bool {
    (*vmm).shutdown.load(Ordering::Relaxed)
}
