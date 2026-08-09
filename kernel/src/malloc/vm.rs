use crate::config::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use crate::context::{CpuLocal, current_thread, uma};
use crate::uma::{Alloc, SlabFlags, UmaFlags, UmaZone};
use crate::vm::{PageObj, Vm, kaddr_to_phys};
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::cell::RefCell;
use core::num::NonZero;

/// Kernel heap that allocate a memory from a virtual memory management system. This struct is a
/// merge of `malloc_type` and `malloc_type_internal` structure.
pub struct VmHeap {
    vm: &'static Vm,
    zones: [Vec<Arc<UmaZone>>; PAGE_SHIFT + 1], // kmemsize + kmemzones
    stats: CpuLocal<RefCell<Stats>>,            // mti_stats
}

impl VmHeap {
    const KMEM_ZSHIFT: usize = 4;
    const KMEM_ZBASE: usize = 16;
    const KMEM_ZMASK: usize = Self::KMEM_ZBASE - 1;
    const KMEM_ZSIZE: usize = PAGE_SIZE.get() >> Self::KMEM_ZSHIFT;

    /// See `kmeminit` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1A4B80|
    pub fn new(vm: &'static Vm) -> Self {
        // The possible of maximum alignment that Layout allowed is a bit before the most
        // significant bit of isize (e.g. 0x4000000000000000 on 64 bit system). So we can use
        // "size_of::<usize>() * 8 - 1" to get the size of array for all possible alignment.
        let uma = uma().unwrap();
        let zones = core::array::from_fn(|align| {
            let mut zones = Vec::with_capacity(Self::KMEM_ZSIZE + 1);
            let mut last = 0;
            let align = align
                .try_into()
                .ok()
                .and_then(|align| 1usize.checked_shl(align))
                .unwrap();

            for i in Self::KMEM_ZSHIFT.. {
                // Stop if size larger than page size.
                let size = NonZero::new(1usize << i).unwrap();

                if size > PAGE_SIZE {
                    break;
                }

                // Create zone.
                let zone = Arc::new(uma.into_owned().create_zone(
                    size.to_string(),
                    size,
                    Some(align - 1),
                    None,
                    UmaFlags::Malloc,
                ));

                while last <= size.get() {
                    zones.push(zone.clone());
                    last += Self::KMEM_ZBASE;
                }
            }

            zones
        });

        Self {
            vm,
            zones,
            stats: CpuLocal::new(|_| RefCell::default()),
        }
    }

    /// Returns null on failure.
    ///
    /// See `malloc` on the Orbis for a reference.
    ///
    /// # Safety
    /// `layout` must be nonzero.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1A4220|
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Our implementation imply M_WAITOK.
        let td = current_thread();

        if !td.can_sleep() {
            panic!("heap allocation in a non-sleeping context is not supported");
        }

        // Determine how to allocate.
        let lock = td.disable_vm_heap();
        let size = layout.size();
        let mem = if size <= PAGE_SIZE.get() {
            // Get zone to allocate from.
            let align = layout.align().trailing_zeros() as usize;
            let size = if (size & Self::KMEM_ZMASK) != 0 {
                // TODO: Refactor this for readability.
                (size + Self::KMEM_ZBASE) & !Self::KMEM_ZMASK
            } else {
                size
            };

            // Allocate a memory from UMA zone.
            let zone = &self.zones[align][size >> Self::KMEM_ZSHIFT];
            let mem = zone.alloc(Alloc::Wait | Alloc::Zero);

            // Update stats.
            let stats = self.stats.lock();
            let mut stats = stats.borrow_mut();
            let size = if mem.is_null() { 0 } else { zone.size().get() };

            if size != 0 {
                stats.alloc_bytes = stats
                    .alloc_bytes
                    .checked_add(size.try_into().unwrap())
                    .unwrap();
                stats.alloc_count += 1;
            }

            // TODO: How to update mts_size here since our zone table also indexed by alignment?
            mem
        } else {
            todo!()
        };

        drop(lock);

        mem
    }

    /// See `free` on the Orbis for a reference.
    ///
    /// # Safety
    /// `ptr` must be obtained with [Self::alloc()] and `layout` must be the same one that was
    /// passed to that method.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1A43E0|
    pub unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        let page = (ptr as usize) & !PAGE_MASK.get();
        let page = unsafe { kaddr_to_phys(page) };
        let page = self.vm.phys_to_page(page).unwrap(); // Orbis assume the pointer is not null.
        let ps = page.state.lock();
        let obj = ps.object.as_ref().unwrap(); // Orbis panic when this is null.
        let PageObj::Slab(slab) = obj;

        if slab.flags().has_any(SlabFlags::Malloc) {
            todo!()
        } else {
            todo!()
        }
    }
}

/// Implementation of `malloc_type_stats` structure.
#[derive(Default)]
struct Stats {
    alloc_bytes: u64, // mts_memalloced
    alloc_count: u64, // mts_numallocs
}
