use super::{MemAffinity, VmPage};
use alloc::collections::vec_deque::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Provides methods to allocate physical memory.
pub struct PhysAllocator {
    nfree: usize, // vm_nfreelists
    #[allow(clippy::type_complexity)] // TODO: Remove this.
    lookup_lists: [Arc<[[[VecDeque<VmPage>; 13]; 3]; 2]>; 2], // vm_phys_lookup_lists
}

impl PhysAllocator {
    /// See `vm_phys_init` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x15F410|
    pub fn new(phys_avail: &[u64; 61], ma: Option<&MemAffinity>) -> Self {
        // Create segments.
        let mut segs = Vec::new();
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
                    Self::create_seg(&mut segs, ma);
                }

                Self::create_seg(&mut segs, ma);

                nfree = 1;
            } else {
                Self::create_seg(&mut segs, ma);
            }
        }

        // Populate vm_phys_free_queues. Do not use Clone to construct the array here since it will
        // refer to the same object.
        let free_queues = [
            Arc::<[[[VecDeque<VmPage>; 13]; 3]; 2]>::default(),
            Arc::<[[[VecDeque<VmPage>; 13]; 3]; 2]>::default(),
        ];

        // Populate vm_phys_lookup_lists.
        let lookup_lists = [free_queues[0].clone(), free_queues[1].clone()];

        Self {
            nfree,
            lookup_lists,
        }
    }

    /// See `vm_phys_alloc_pages` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x160520|
    pub fn alloc_page(&self, vm: usize, pool: usize, order: usize) -> Option<VmPage> {
        // TODO: There is an increasement on unknown variable here.
        let mut i = 0;

        loop {
            let l = &self.lookup_lists[i];

            if let Some(v) = self.alloc_freelist(&l[vm], pool, order) {
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
    fn alloc_freelist(
        &self,
        list: &[[VecDeque<VmPage>; 13]; 3],
        pool: usize,
        order: usize,
    ) -> Option<VmPage> {
        if order >= 13 {
            return None;
        }

        let mut i = 0;

        loop {
            match list[pool][order + i].front() {
                Some(v) => v,
                None => match (order + i) < 12 {
                    true => {
                        i += 1;
                        continue;
                    }
                    false => break,
                },
            };

            todo!()
        }

        let mut next = 11;

        loop {
            let mut found = None;

            for f in list {
                found = f[next + 1].front();

                if found.is_some() {
                    break;
                }
            }

            match found {
                Some(_) => todo!(),
                None => {
                    if next < order || next == 0 {
                        break;
                    }

                    next -= 1;
                }
            }
        }

        None
    }

    /// See `vm_phys_create_seg` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x15F8A0|
    fn create_seg(segs: &mut Vec<PhysSeg>, ma: Option<&MemAffinity>) {
        match ma {
            Some(_) => todo!(),
            None => segs.push(PhysSeg {}),
        }
    }
}

/// Implementation of `vm_phys_seg` structure.
struct PhysSeg {}
