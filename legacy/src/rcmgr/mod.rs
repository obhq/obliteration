use std::sync::Arc;

/// Implementation of RcMgr kernel services.
///
/// Not sure what the meaning of "Rc".
pub struct RcMgr {}

impl RcMgr {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    /// See `sceSblRcMgrIsAllowULDebugger` on the PS4 for a reference.
    pub fn is_allow_ul_debugger(&self) -> bool {
        if !self.is_qa_enabled() {
            return false;
        }

        todo!()
    }

    /// See `sceSblRcMgrIsSoftwagnerQafForAcmgr` on the PS4 for a reference,
    pub fn is_softwagner_qaf_for_acmgr(&self) -> bool {
        if !self.is_qa_enabled() {
            return false;
        }

        todo!()
    }

    fn is_qa_enabled(&self) -> bool {
        false
    }
}
