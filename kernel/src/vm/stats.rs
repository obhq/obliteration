use crate::lock::Gutex;

/// Contains statistics for a VM.
///
/// This is a subset of `vmmeter` structure.
pub struct VmStats {
    pub free_reserved: usize,             // v_free_reserved
    pub cache_count: Gutex<usize>,        // v_cache_count
    pub free_count: Gutex<usize>,         // v_free_count
    pub interrupt_free_min: Gutex<usize>, // v_interrupt_free_min
}
