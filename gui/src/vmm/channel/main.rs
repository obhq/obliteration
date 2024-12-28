use gdbstub::stub::MultiThreadStopReason;
use obconf::ConsoleType;
use std::error::Error;

/// Provides method to send and receive events from the main thread.
pub struct MainStream {}

impl MainStream {
    pub(super) fn new() -> Self {
        Self {}
    }

    pub fn error(&self, reason: impl Error + Send + Sync + 'static) {
        todo!()
    }

    pub fn log(&self, ty: ConsoleType, msg: impl Into<String>) {
        todo!()
    }

    pub fn breakpoint(&self, stop: Option<MultiThreadStopReason<u64>>) -> BreakpointLock {
        todo!()
    }

    pub fn exit(&self, success: bool) {
        todo!()
    }
}

/// This struct will prevent the other CPU from entering a debugger dispatch loop.
pub struct BreakpointLock {}
