use crate::MemoryInfo;
use alloc::sync::Arc;

/// Implementation of Direct Memory system.
pub struct Dmem {}

impl Dmem {
    /// See `initialize_dmem` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F5C20|
    pub fn new(mi: &mut MemoryInfo) -> Arc<Self> {
        // TODO: Figure out what the purpose of this 16MB block of memory.
        if Self::reserve_phys(mi, 0x1000000, 1) == 0 {
            panic!("no available memory for high-address memory");
        }

        todo!()
    }

    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F64C0|
    fn reserve_phys(mi: &mut MemoryInfo, size: u64, align: i64) -> u64 {
        let mut i = mi.physmap_last;

        loop {
            let start = mi.physmap[i];
            let end = mi.physmap[i + 1];
            let addr = (end - size) & ((-align) as u64);

            if addr >= start {
                let aligned_end = addr + size;

                // Check if this take the whole block.
                if (addr == start) && (aligned_end == end) {
                    mi.physmap.copy_within((i + 2).., i);
                    mi.physmap_last -= 2;
                    return addr;
                }

                // Check if this create a hole in the block.
                if (addr != start) && (aligned_end != end) {
                    mi.physmap.copy_within(i..(mi.physmap_last + 2), i + 2);
                    mi.physmap[i + 1] = addr;
                    mi.physmap[i + 2] = aligned_end;
                    mi.physmap_last += 2;
                    return addr;
                }

                // Check if this take the end of the block.
                if addr != start {
                    assert_eq!(aligned_end, end);
                    mi.physmap[i + 1] = addr;
                    return addr;
                }

                // Take the start of the block.
                mi.physmap[i] = aligned_end;

                return addr;
            }

            // Move to lower map.
            if i < 2 {
                break 0;
            }

            i -= 2;
        }
    }
}
