use super::VmPage;

/// Provides methods to allocate physical memory.
pub struct PhysAllocator {
    nfree: u8, // vm_nfreelists
}

impl PhysAllocator {
    /// See `vm_phys_init` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x15F410|
    pub fn new(phys_avail: &[u64; 61]) -> Self {
        let mut nfree = 0;

        for i in (0..).step_by(2) {
            // Check if end entry.
            let addr = phys_avail[i];
            let end = phys_avail[i + 1];

            if end == 0 {
                break;
            }

            // TODO: What is 16777216?
            if addr < 16777216 {
                let unk = end < 0x1000001;

                if !unk {
                    // TODO: Invoke vm_phys_create_seg.
                }

                // TODO: Invoke vm_phys_create_seg.
                nfree = 1;
            } else {
                // TODO: Invoke vm_phys_create_seg.
            }
        }

        Self { nfree }
    }

    /// See `vm_phys_alloc_pages` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x160520|
    pub fn alloc_page(&self) -> Option<VmPage> {
        // TODO: There is an increasement on unknown variable here.
        let mut i = 0;

        loop {
            if let Some(v) = self.alloc_freelist() {
                return Some(v);
            }

            i += 1;

            if i >= (self.nfree + 1) {
                break;
            }
        }

        None
    }

    /// See `vm_phys_alloc_freelist_pages` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1605D0|
    fn alloc_freelist(&self) -> Option<VmPage> {
        todo!()
    }
}
