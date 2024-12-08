// SPDX-License-Identifier: MIT OR Apache-2.0
use super::cpu::{CpuManager, GdbError};
use super::VmmHandler;
use crate::debug::DebugClient;
use crate::hv::Hypervisor;
use gdbstub::stub::state_machine::state::{Idle, Running};
use gdbstub::stub::state_machine::{GdbStubStateMachine, GdbStubStateMachineInner};
use gdbstub::stub::MultiThreadStopReason;
use thiserror::Error;

impl<'a, 'b, H: Hypervisor, E: VmmHandler> CpuManager<'a, 'b, H, E> {
    pub(super) fn dispatch_gdb_idle(
        &mut self,
        mut state: GdbStubStateMachineInner<
            'static,
            Idle<CpuManager<'a, 'b, H, E>>,
            CpuManager<'a, 'b, H, E>,
            DebugClient,
        >,
    ) -> Result<
        Result<
            GdbStubStateMachine<'static, CpuManager<'a, 'b, H, E>, DebugClient>,
            GdbStubStateMachineInner<
                'static,
                Idle<CpuManager<'a, 'b, H, E>>,
                CpuManager<'a, 'b, H, E>,
                DebugClient,
            >,
        >,
        DispatchGdbIdleError,
    > {
        let b = match state.borrow_conn().read() {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Err(state)),
            Err(e) => return Err(DispatchGdbIdleError::ReadData(e)),
        };

        state
            .incoming_data(self, b)
            .map(Ok)
            .map_err(DispatchGdbIdleError::ProcessData)
    }

    pub(super) fn dispatch_gdb_running(
        &mut self,
        mut state: GdbStubStateMachineInner<
            'static,
            Running,
            CpuManager<'a, 'b, H, E>,
            DebugClient,
        >,
        stop: Option<MultiThreadStopReason<u64>>,
    ) -> Result<
        Result<
            GdbStubStateMachine<'static, CpuManager<'a, 'b, H, E>, DebugClient>,
            GdbStubStateMachineInner<'static, Running, CpuManager<'a, 'b, H, E>, DebugClient>,
        >,
        DispatchGdbRunningError,
    > {
        // Check If we are here because of a breakpoint.
        if let Some(r) = stop {
            return state
                .report_stop(self, r)
                .map(Ok)
                .map_err(DispatchGdbRunningError::ReportStopReason);
        }

        // Check for pending command.
        let b = match state.borrow_conn().read() {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Err(state)),
            Err(e) => return Err(DispatchGdbRunningError::ReadData(e)),
        };

        state
            .incoming_data(self, b)
            .map(Ok)
            .map_err(DispatchGdbRunningError::ProcessData)
    }
}

#[derive(Debug, Error)]
pub(super) enum DispatchGdbIdleError {
    #[error("couldn't read data from the debugger")]
    ReadData(#[source] std::io::Error),

    #[error("couldn't process data from the debugger")]
    ProcessData(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),
}

#[derive(Debug, Error)]
pub(super) enum DispatchGdbRunningError {
    #[error("couldn't report stop reason to the debugger")]
    ReportStopReason(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),

    #[error("couldn't read data from the debugger")]
    ReadData(#[source] std::io::Error),

    #[error("couldn't process data from the debugger")]
    ProcessData(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),
}
