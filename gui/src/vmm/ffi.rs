use super::{DebugResult, DispatchDebugResult, KernelStop, Vmm};
use crate::error::RustError;
use gdbstub::stub::state_machine::GdbStubStateMachine;
use std::sync::atomic::Ordering;

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
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
