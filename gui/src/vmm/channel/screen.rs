use gdbstub::stub::MultiThreadStopReason;

/// Provides method to send and receive events from the screen.
pub struct ScreenStream {}

impl ScreenStream {
    pub(super) fn new() -> Self {
        Self {}
    }

    pub fn recv(&self) {}

    pub fn breakpoint(&self, stop: Option<MultiThreadStopReason<u64>>) -> BreakpointLock {
        BreakpointLock {}
    }
}

/// This struct will prevent the other CPU from entering a debugger dispatch loop.
pub struct BreakpointLock {}
