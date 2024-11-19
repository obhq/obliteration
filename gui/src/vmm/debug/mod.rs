// SPDX-License-Identifier: MIT OR Apache-2.0
use super::cpu::CpuManager;
use crate::debug::DebugClient;
use crate::error::RustError;
use crate::hv::Hypervisor;
use crate::screen::Screen;
use gdbstub::stub::state_machine::state::{Idle, Running};
use gdbstub::stub::state_machine::{GdbStubStateMachine, GdbStubStateMachineInner};
use gdbstub::stub::MultiThreadStopReason;

pub fn dispatch_idle<H: Hypervisor, S: Screen>(
    target: &mut CpuManager<H, S>,
    mut state: GdbStubStateMachineInner<
        'static,
        Idle<CpuManager<H, S>>,
        CpuManager<H, S>,
        DebugClient,
    >,
) -> Result<
    Result<
        GdbStubStateMachine<'static, CpuManager<H, S>, DebugClient>,
        GdbStubStateMachineInner<'static, Idle<CpuManager<H, S>>, CpuManager<H, S>, DebugClient>,
    >,
    RustError,
> {
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

pub fn dispatch_running<H: Hypervisor, S: Screen>(
    target: &mut CpuManager<H, S>,
    mut state: GdbStubStateMachineInner<'static, Running, CpuManager<H, S>, DebugClient>,
    stop: Option<MultiThreadStopReason<u64>>,
) -> Result<
    Result<
        GdbStubStateMachine<'static, CpuManager<H, S>, DebugClient>,
        GdbStubStateMachineInner<'static, Running, CpuManager<H, S>, DebugClient>,
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
