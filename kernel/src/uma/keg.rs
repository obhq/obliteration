use super::arch::small_alloc;
use super::{Alloc, FreeItem, Slab, SlabHdr, Uma, UmaFlags};
use crate::config::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use crate::lock::Mutex;
use crate::vm::Vm;
use alloc::collections::vec_deque::VecDeque;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::cmp::{max, min};
use core::num::NonZero;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

/// Implementation of `uma_keg` structure.
pub struct UmaKeg<T> {
    vm: Arc<Vm>,
    size: NonZero<usize>,             // uk_size
    rsize: usize,                     // uk_rsize
    pgoff: usize,                     // uk_pgoff
    ppera: usize,                     // uk_ppera
    ipers: usize,                     // uk_ipers
    alloc: fn(&Vm, Alloc) -> *mut u8, // uk_allocf
    init: Option<fn()>,               // uk_init
    max_pages: usize,                 // uk_maxpages
    recurse: AtomicU32,               // uk_recurse
    flags: UmaFlags,                  // uk_flags
    state: Mutex<KegState<T>>,
}

impl<T: FreeItem> UmaKeg<T> {
    /// `align` is the actual alignment **minus** one, which mean if you want each item to be 8
    /// bytes alignment this value will be 7.
    ///
    /// See `keg_ctor` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13CF40|
    pub(super) fn new(
        vm: Arc<Vm>,
        size: NonZero<usize>,
        align: usize,
        init: Option<fn()>,
        mut flags: UmaFlags,
    ) -> Self {
        if flags.has_any(UmaFlags::Vm) {
            todo!()
        }

        if flags.has_any(UmaFlags::ZInit) {
            todo!()
        }

        if flags.has_any(UmaFlags::Malloc) {
            flags |= UmaFlags::VToSlab;
        }

        flags |= T::flags();

        // Get header layout.
        let hdr = Layout::new::<SlabHdr<T>>();
        let (mut hdr, off) = hdr.extend(Layout::new::<T>()).unwrap();

        hdr = hdr.pad_to_align();

        // Get UMA_FRITM_SZ and UMA_FRITMREF_SZ.
        let free_item = hdr.size() - off;
        let available = PAGE_SIZE.get() - hdr.size();

        // Get uk_rsize, uk_ppera and uk_ipers.
        let (rsize, ppera, ipers) = if flags.has_any(UmaFlags::CacheSpread) {
            // Get uk_rsize.
            let rsize = size.get().next_multiple_of(align + 1);
            let align = align + 1;
            let rsize = if (rsize & align) == 0 {
                // TODO: What is this?
                rsize + align
            } else {
                rsize
            };

            // Get uk_ppera.
            let pages = (PAGE_SIZE.get() / align * rsize) >> PAGE_SHIFT;
            let ppera = min(pages, (128 * 1024) / PAGE_SIZE);

            // TODO: Why we need to add the differences to the calculation?
            let ipers = (ppera * PAGE_SIZE.get() + (rsize - size.get())) / rsize;

            (rsize, ppera, ipers)
        } else {
            // TODO: Not sure why we need space at least for 2 free item?
            if (size.get() + free_item) > available {
                // TODO: Set uk_ppera and uk_rsize.
                if !flags.has_any(UmaFlags::Internal) {
                    flags |= UmaFlags::Offpage;

                    if !flags.has_any(UmaFlags::VToSlab) {
                        flags |= UmaFlags::Hash;
                    }
                }

                // Get uk_ppera.
                let mut ppera = size.get() >> PAGE_SHIFT;

                if size.get() > (size.get() & !PAGE_MASK.get()) {
                    ppera += 1;
                }

                (size.get(), ppera, 1)
            } else {
                // Get uk_rsize.
                let rsize = max(size, Uma::SMALLEST_UNIT);
                let rsize = rsize.get().next_multiple_of(align + 1);

                // Get uk_ipers.
                let mut ipers = available / (rsize + free_item);

                // TODO: Verify if this valid for PAGE_SIZE < 0x4000.
                if !flags.has_any(UmaFlags::Internal | UmaFlags::CacheOnly)
                    && (available % (rsize + free_item)) >= Uma::MAX_WASTE.get()
                    && (PAGE_SIZE.get() / rsize) > ipers
                {
                    ipers = PAGE_SIZE.get() / rsize;

                    if flags.has_any(UmaFlags::VToSlab) {
                        flags |= UmaFlags::Offpage;
                    } else {
                        flags |= UmaFlags::Offpage | UmaFlags::Hash;
                    }
                }

                (rsize, 1, ipers)
            }
        };

        if flags.has_any(UmaFlags::Offpage) {
            // TODO: Set uk_slabzone.
        }

        // Get allocator.
        let alloc = if ppera == 1 {
            // TODO: Get uk_freef.
            small_alloc
        } else {
            Self::page_alloc
        };

        if flags.has_any(UmaFlags::MtxClass) {
            todo!()
        }

        // Get uk_pgoff.
        let mut pgoff = 0;

        if !flags.has_any(UmaFlags::Offpage) {
            let space = ppera * PAGE_SIZE.get();

            // TODO: This can cause a pointer to slab unaligned.
            pgoff = (space - hdr.size()) - ipers * free_item;

            // TODO: What is this?
            if space < pgoff + hdr.size() + ipers * free_item {
                panic!("UMA slab won't fit");
            }
        }

        if flags.has_any(UmaFlags::Hash) {
            todo!()
        }

        // TODO: Add uk_zones.
        // TODO: Add uma_kegs.
        Self {
            vm,
            size,
            rsize,
            pgoff,
            ppera,
            ipers,
            alloc,
            init,
            max_pages: 0,
            recurse: AtomicU32::new(0),
            flags,
            state: Mutex::new(KegState {
                pages: 0,
                free: 0,
                partial_slabs: VecDeque::new(),
            }),
        }
    }
}

impl<T> UmaKeg<T> {
    pub fn size(&self) -> NonZero<usize> {
        self.size
    }

    pub fn allocated_size(&self) -> usize {
        self.rsize
    }

    pub fn item_per_slab(&self) -> usize {
        self.ipers
    }

    pub fn recurse(&self) -> u32 {
        self.recurse.load(Ordering::Relaxed)
    }

    pub fn flags(&self) -> UmaFlags {
        self.flags
    }

    /// See `page_alloc` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1402F0|
    fn page_alloc(_: &Vm, _: Alloc) -> *mut u8 {
        todo!()
    }
}

impl<T: FreeItem> UmaKeg<T> {
    /// Unlike Orbis, our slab contains a strong reference to its keg. That mean all allocated slabs
    /// need to free manually otherwise the keg will be leak.
    ///
    /// See `keg_fetch_slab` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x141E20|
    pub fn fetch_slab(self: &Arc<Self>, mut flags: Alloc) -> Option<NonNull<Slab<T>>> {
        let mut state = self.state.lock();

        while state.free == 0 {
            if flags.has_any(Alloc::NoVm) {
                return None;
            }

            #[allow(clippy::while_immutable_condition)] // TODO: Remove this.
            while self.max_pages != 0 && self.max_pages <= state.pages {
                todo!()
            }

            self.recurse.fetch_add(1, Ordering::Relaxed);
            let slab = self.alloc_slab(&mut state, flags);
            self.recurse.fetch_sub(1, Ordering::Relaxed);

            if let Some(slab) = NonNull::new(slab) {
                state.partial_slabs.push_front(slab);
                return Some(slab);
            }

            flags |= Alloc::NoVm;
        }

        todo!()
    }

    /// See `keg_alloc_slab` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13FBA0|
    fn alloc_slab(self: &Arc<Self>, state: &mut KegState<T>, flags: Alloc) -> *mut Slab<T> {
        if self.flags.has_any(UmaFlags::Offpage) {
            todo!()
        } else {
            // Get allocation flags.
            let flags = if self.flags.has_any(UmaFlags::Malloc) {
                flags & !Alloc::Zero
            } else {
                flags | Alloc::Zero
            };

            // Allocate.
            let mem = (self.alloc)(&self.vm, flags);

            if !mem.is_null() {
                // The Orbis also check if uk_flags does not contains UMA_ZONE_OFFPAGE, which seems
                // to be useless since we only be here when it does not contains UMA_ZONE_OFFPAGE.
                let hdr = unsafe { mem.byte_add(self.pgoff).cast::<SlabHdr<T>>() };

                if self.flags.has_any(UmaFlags::VToSlab) && self.ppera != 0 {
                    todo!()
                }

                // TODO: I'm not confident about the memory layout here. The variables calculation
                // during keg construction is very complicated and I don't fully understand it. If
                // we encounter some memory corruptions then this is likely to be the root of
                // problem.
                let v = SlabHdr {
                    keg: self.clone(),
                    free_count: self.ipers,
                    first_free: 0,
                    items: mem,
                };

                unsafe { hdr.write(v) };

                // Initialize free items. The offset calculation here should be optimized away.
                let (_, off) = Layout::new::<SlabHdr<T>>()
                    .extend(Layout::new::<T>())
                    .unwrap();
                let free = unsafe { hdr.byte_add(off).cast::<T>() };

                for i in 0..self.ipers {
                    let item = T::new(i);

                    unsafe { free.add(i).write(item) };
                }

                if self.init.is_some() {
                    todo!()
                }

                if self.flags.has_any(UmaFlags::Hash) {
                    todo!()
                }

                state.pages += self.ppera;
                state.free += self.ipers;

                return core::ptr::slice_from_raw_parts_mut(hdr, self.ipers) as *mut Slab<T>;
            }

            todo!()
        }
    }
}

/// Mutable state of [UmaKeg].
struct KegState<T> {
    pages: usize,                              // uk_pages
    free: usize,                               // uk_free
    partial_slabs: VecDeque<NonNull<Slab<T>>>, // uk_part_slab
}

unsafe impl<T: Send> Send for KegState<T> {}
