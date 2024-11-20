// SPDX-License-Identifier: MIT OR Apache-2.0
use super::cpu::{CpuManager, GdbError};
use crate::debug::DebugClient;
use crate::hv::Hypervisor;
use crate::screen::Screen;
use gdbstub::stub::state_machine::state::{Idle, Running};
use gdbstub::stub::state_machine::{GdbStubStateMachine, GdbStubStateMachineInner};
use gdbstub::stub::MultiThreadStopReason;
use thiserror::Error;

impl<H: Hypervisor, S: Screen> CpuManager<H, S> {
    pub fn dispatch_idle(
        &mut self,
        mut state: GdbStubStateMachineInner<
            'static,
            Idle<CpuManager<H, S>>,
            CpuManager<H, S>,
            DebugClient,
        >,
    ) -> Result<
        Result<
            GdbStubStateMachine<'static, CpuManager<H, S>, DebugClient>,
            GdbStubStateMachineInner<
                'static,
                Idle<CpuManager<H, S>>,
                CpuManager<H, S>,
                DebugClient,
            >,
        >,
        DispatchIdleError,
    > {
        let b = match state.borrow_conn().read() {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Err(state)),
            Err(e) => return Err(DispatchIdleError::ReadData(e)),
        };

        state
            .incoming_data(self, b)
            .map(Ok)
            .map_err(DispatchIdleError::ProcessData)
    }

    pub fn dispatch_running(
        &mut self,
        mut state: GdbStubStateMachineInner<'static, Running, CpuManager<H, S>, DebugClient>,
        stop: Option<MultiThreadStopReason<u64>>,
    ) -> Result<
        Result<
            GdbStubStateMachine<'static, CpuManager<H, S>, DebugClient>,
            GdbStubStateMachineInner<'static, Running, CpuManager<H, S>, DebugClient>,
        >,
        DispatchRunningError,
    > {
        // Check If we are here because of a breakpoint.
        if let Some(r) = stop {
            return state
                .report_stop(self, r)
                .map(Ok)
                .map_err(DispatchRunningError::ReportStopReason);
        }

        // Check for pending command.
        let b = match state.borrow_conn().read() {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Err(state)),
            Err(e) => return Err(DispatchRunningError::ReadData(e)),
        };

        state
            .incoming_data(self, b)
            .map(Ok)
            .map_err(DispatchRunningError::ProcessData)
    }
}

#[derive(Debug, Error)]
pub enum DispatchIdleError {
    #[error("couldn't read data from the debugger")]
    ReadData(#[source] std::io::Error),

    #[error("couldn't process data from the debugger")]
    ProcessData(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),
}

#[derive(Debug, Error)]
pub enum DispatchRunningError {
    #[error("couldn't report stop reason to the debugger")]
    ReportStopReason(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),

    #[error("couldn't read data from the debugger")]
    ReadData(#[source] std::io::Error),

    #[error("couldn't process data from the debugger")]
    ProcessData(#[source] gdbstub::stub::GdbStubError<GdbError, std::io::Error>),
}
