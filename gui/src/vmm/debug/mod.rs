// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

use crate::debug::Debugger;
use crate::error::RustError;
use gdbstub::stub::state_machine::state::{Idle, Running};
use gdbstub::stub::state_machine::{GdbStubStateMachine, GdbStubStateMachineInner};
use gdbstub::stub::MultiThreadStopReason;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

pub fn dispatch_idle(
    target: &mut Target,
    mut state: GdbStubStateMachineInner<'static, Idle<Target>, Target, Debugger>,
) -> Result<GdbStubStateMachine<'static, Target, Debugger>, RustError> {
    let b = state
        .borrow_conn()
        .read()
        .map_err(|e| RustError::with_source("couldn't read data from the debugger", e))?;

    state
        .incoming_data(target, b)
        .map_err(|e| RustError::with_source("couldn't process data from the debugger", e))
}

pub fn dispatch_running(
    target: &mut Target,
    mut state: GdbStubStateMachineInner<'static, Running, Target, Debugger>,
    stop: Option<MultiThreadStopReason<u64>>,
) -> Result<
    Result<
        GdbStubStateMachine<'static, Target, Debugger>,
        GdbStubStateMachineInner<'static, Running, Target, Debugger>,
    >,
    RustError,
> {
    // Check If we are here because of a breakpoint.
    if let Some(r) = stop {
        return state
            .report_stop(target, r)
            .map(Ok)
            .map_err(|e| RustError::with_source("couldn't report stop reason to the debugger", e));
    }

    // Check for pending command.
    let b = match state.borrow_conn().read() {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Err(state)),
        Err(e) => {
            return Err(RustError::with_source(
                "couldn't read data from the debugger",
                e,
            ));
        }
    };

    state
        .incoming_data(target, b)
        .map(Ok)
        .map_err(|e| RustError::with_source("couldn't process data from the debugger", e))
}
