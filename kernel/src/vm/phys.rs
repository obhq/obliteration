use super::{MemAffinity, VmPage};
use crate::config::PAGE_SHIFT;
use crate::lock::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use indexmap::IndexSet;
use rustc_hash::FxBuildHasher;

/// Provides methods to allocate physical memory.
pub struct PhysAllocator {
    segs: Vec<PhysSeg>, // vm_phys_segs + vm_phys_nsegs
    nfree: usize,       // vm_nfreelists
    lookup_lists: [Arc<Mutex<[[[IndexSet<Arc<VmPage>, FxBuildHasher>; 13]; 3]; 2]>>; 2], // vm_phys_lookup_lists
}

impl PhysAllocator {
    /// See `vm_phys_init` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x15F410|
    pub fn new(phys_avail: &[u64; 61], ma: Option<&MemAffinity>) -> Self {
        // Populate vm_phys_free_queues. Do not use Clone to construct the array here since it will
        // refer to the same object. The Orbis do this after segments creation but we do it before
        // instead.
        let free_queues = [
            Arc::<Mutex<[[[IndexSet<Arc<VmPage>, FxBuildHasher>; 13]; 3]; 2]>>::default(),
            Arc::<Mutex<[[[IndexSet<Arc<VmPage>, FxBuildHasher>; 13]; 3]; 2]>>::default(),
        ];

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
                    Self::create_seg(&mut segs, ma, &free_queues, addr, 0x1000000, 1);

                    // The Orbis also update end address here but it seems like the value is always
                    // the same as current value.
                    addr = 0x1000000;
                }

                Self::create_seg(&mut segs, ma, &free_queues, addr, end, unk.into());

                nfree = 1;
            } else {
                Self::create_seg(&mut segs, ma, &free_queues, addr, end, 0);
            }
        }

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
        let mut flind = 0;

        loop {
            let l = self.lookup_lists[flind].lock();

            if let Some(v) = self.alloc_freelist(&l[vm], pool, order) {
                return Some(v);
            }

            flind += 1;

            if flind >= (self.nfree + 1) {
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
        list: &[[IndexSet<Arc<VmPage>, FxBuildHasher>; 13]; 3],
        pool: usize,
        order: usize,
    ) -> Option<VmPage> {
        // Beware for deadlock here since we currently own a lock to free queue.
        if order >= 13 {
            return None;
        }

        let mut i = 0;

        loop {
            match list[pool][order + i].first() {
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
                found = f[next + 1].first();

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
    fn create_seg(
        segs: &mut Vec<PhysSeg>,
        ma: Option<&MemAffinity>,
        queues: &[Arc<Mutex<[[[IndexSet<Arc<VmPage>, FxBuildHasher>; 13]; 3]; 2]>>; 2],
        start: u64,
        end: u64,
        flind: usize,
    ) {
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
                    free_queues: queues[flind].clone(),
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
    pub free_queues: Arc<Mutex<[[[IndexSet<Arc<VmPage>, FxBuildHasher>; 13]; 3]; 2]>>, // free_queues
}
