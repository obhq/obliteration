use crate::vmm::VmmEvent;

/// Provides method to send and receive events from the VMM.
pub struct VmmStream {}

impl VmmStream {
    pub(super) fn new() -> Self {
        Self {}
    }

    pub async fn recv(&mut self) -> Option<VmmEvent> {
        todo!()
    }
}
