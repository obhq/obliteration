/// An implementation of `ucred` structure.
#[derive(Debug)]
pub struct Ucred {}

impl Ucred {
    pub fn new() -> Self {
        Self {}
    }

    /// See `sceSblACMgrIsWebcoreProcess` on the PS4 for a reference.
    pub fn is_webcore_process(&self) -> bool {
        // TODO: Implement this.
        false
    }

    /// See `sceSblACMgrIsDiskplayeruiProcess` on the PS4 for a reference.
    pub fn is_diskplayerui_process(&self) -> bool {
        // TODO: Implement this.
        false
    }

    /// See `sceSblACMgrIsNongameUcred` on the PS4 for a reference.
    pub fn is_nongame(&self) -> bool {
        // TODO: Implement this.
        false
    }
}
