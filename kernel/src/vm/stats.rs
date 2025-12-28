/// Contains statistics for a VM.
///
/// This struct **MUST** be locked before other VM locks.
///
/// This is a subset of `vmmeter` structure.
pub struct VmStats {
    pub free_reserved: usize,      // v_free_reserved
    pub cache_min: usize,          // v_cache_min
    pub cache_count: usize,        // v_cache_count
    pub free_count: usize,         // v_free_count
    pub interrupt_free_min: usize, // v_interrupt_free_min
    pub wire_count: usize,         // v_wire_count
}
