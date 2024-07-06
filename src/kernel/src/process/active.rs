/// States of a process when it is still active.
#[derive(Debug)]
pub struct ActiveProc {}

impl ActiveProc {
    pub(super) fn new() -> Self {
        Self {}
    }
}
