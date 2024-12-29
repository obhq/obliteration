use gdbstub::stub::MultiThreadStopReason;
use obconf::ConsoleType;

/// Provides method to send and receive events from the main thread.
pub struct MainStream {}

impl MainStream {
    pub(super) fn new() -> Self {
        Self {}
    }

    pub fn log(&self, ty: ConsoleType, msg: impl Into<String>) {
        todo!()
    }

    pub fn breakpoint(&self, stop: Option<MultiThreadStopReason<u64>>) -> BreakpointLock {
        todo!()
    }
}

/// This struct will prevent the other CPU from entering a debugger dispatch loop.
pub struct BreakpointLock {}
