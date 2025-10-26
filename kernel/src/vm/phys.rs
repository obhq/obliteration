use super::{MemAffinity, VmPage};
use crate::config::PAGE_SHIFT;
use alloc::collections::vec_deque::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Provides methods to allocate physical memory.
pub struct PhysAllocator {
    segs: Vec<PhysSeg>, // vm_phys_segs + vm_phys_nsegs
    nfree: usize,       // vm_nfreelists
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
            let mut addr = phys_avail[i];
            let end = phys_avail[i + 1];

            if end == 0 {
                break;
            }

            // TODO: Why Orbis need to create 16MB segment here?
            if addr < 16777216 {
                let unk = end < 0x1000001;

                if !unk {
                    Self::create_seg(&mut segs, ma, addr, 0x1000000);

                    // The Orbis also update end address here but it seems like the value is always
                    // the same as current value.
                    addr = 0x1000000;
                }

                Self::create_seg(&mut segs, ma, addr, end);

                nfree = 1;
            } else {
                Self::create_seg(&mut segs, ma, addr, end);
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
            segs,
            nfree,
            lookup_lists,
        }
    }

    /// # Panics
    /// If `i` is not valid.
    pub fn segment(&self, i: usize) -> &PhysSeg {
        &self.segs[i]
    }

    /// See `vm_phys_paddr_to_segind` on the Orbis for a reference. Our implementation is a bit
    /// differences here. Orbis will panic if segment not found but we return [None] instead.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x15FC40|
    pub fn segment_index(&self, pa: u64) -> Option<usize> {
        for (i, s) in self.segs.iter().enumerate() {
            if pa < s.start || pa >= s.end {
                continue;
            }

            return Some(i);
        }

        None
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
    fn create_seg(segs: &mut Vec<PhysSeg>, ma: Option<&MemAffinity>, start: u64, end: u64) {
        match ma {
            Some(_) => todo!(),
            None => {
                let mut first_page = 0;

                for s in segs.iter() {
                    first_page += (s.end - s.start) >> PAGE_SHIFT;
                }

                segs.push(PhysSeg {
                    start,
                    end,
                    first_page: first_page.try_into().unwrap(),
                });
            }
        }
    }
}

/// Implementation of `vm_phys_seg` structure.
pub struct PhysSeg {
    pub start: u64,        // start
    pub end: u64,          // end
    pub first_page: usize, // first_page
}
