use crate::lock::Gutex;
use core::sync::atomic::AtomicUsize;

/// Contains statistics for a VM.
///
/// This struct **MUST** be locked before other VM locks.
///
/// This is a subset of `vmmeter` structure.
pub struct VmStats {
    pub free_reserved: usize,             // v_free_reserved
    pub cache_count: Gutex<usize>,        // v_cache_count
    pub free_count: Gutex<usize>,         // v_free_count
    pub interrupt_free_min: Gutex<usize>, // v_interrupt_free_min
    pub wire_count: AtomicUsize,          // v_wire_count
}
