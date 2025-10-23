use super::{MemAffinity, VmPage};
use crate::config::PAGE_SHIFT;
use alloc::collections::vec_deque::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Provides methods to allocate physical memory.
pub struct PhysAllocator {
    pages: Vec<Arc<VmPage>>, // vm_page_array + vm_page_array_size
    segs: Vec<PhysSeg>,      // vm_phys_segs + vm_phys_nsegs
    nfree: usize,            // vm_nfreelists
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
        // Get vm_page_array_size. The orbis do this in vm_page_startup but we do it here instead.
        let mut pages = 0;

        for i in (0..).step_by(2) {
            let end = phys_avail[i + 1];

            if end == 0 {
                break;
            }

            pages += (end - phys_avail[i]) >> PAGE_SHIFT;
        }

        // Populate vm_page_array. The orbis do this in vm_page_startup but we do it here instead.
        let pages = pages.try_into().unwrap();
        let mut all_pages = Vec::with_capacity(pages);

        for _ in 0..pages {
            all_pages.push(Arc::new(VmPage::new()));
        }

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
            pages: all_pages,
            segs,
            nfree,
            lookup_lists,
        }
    }

    /// See `vm_phys_paddr_to_vm_page` on the Orbis for a reference.
    ///
    /// This function was inlined in `vm_phys_add_page` on the Orbis.
    pub fn page_for(&self, pa: u64) -> Option<&Arc<VmPage>> {
        for s in &self.segs {
            // Check if address within this segment.
            if pa < s.start || pa >= s.end {
                continue;
            }

            // Get page index.
            let i = s.first_page + usize::try_from((pa - s.start) >> PAGE_SHIFT).unwrap();

            return Some(&self.pages[i]);
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
struct PhysSeg {
    start: u64,
    end: u64,
    first_page: usize,
}
