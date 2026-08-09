use super::{Alloc, Slab, SlabFlags, SlabHdr, Uma, UmaFlags, small_alloc};
use crate::config::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use crate::mem::Strong;
use crate::vm::{PageObj, Vm, kaddr_to_phys};
use alloc::collections::vec_deque::VecDeque;
use core::alloc::Layout;
use core::cmp::{max, min};
use core::num::NonZero;
use core::pin::Pin;
use core::ptr::NonNull;

/// Implementation of `uma_keg` structure.
pub struct UmaKeg {
    vm: &'static Vm,
    size: NonZero<usize>,                                  // uk_size
    rsize: usize,                                          // uk_rsize
    pgoff: usize,                                          // uk_pgoff
    ppera: usize,                                          // uk_ppera
    ipers: usize,                                          // uk_ipers
    alloc: fn(&'static Vm, Alloc) -> (*mut u8, SlabFlags), // uk_allocf
    init: Option<fn()>,                                    // uk_init
    max_pages: usize,                                      // uk_maxpages
    pages: usize,                                          // uk_pages
    pub(super) free: usize,                                // uk_free
    recurse: u32,                                          // uk_recurse
    partial_slabs: VecDeque<NonNull<Slab>>,                // uk_part_slab
    flags: UmaFlags,                                       // uk_flags
}

impl UmaKeg {
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
        vm: &'static Vm,
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

        // Get header layout.
        let hdr = Layout::new::<SlabHdr>();
        let (mut hdr, off) = hdr.extend(Layout::new::<u8>()).unwrap();

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
            pages: 0,
            free: 0,
            recurse: 0,
            partial_slabs: VecDeque::new(),
            flags,
        }
    }

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
        self.recurse
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
    fn page_alloc(_: &'static Vm, _: Alloc) -> (*mut u8, SlabFlags) {
        todo!()
    }

    /// See `keg_fetch_slab` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x141E20|
    pub unsafe fn fetch_slab(&mut self, mut flags: Alloc) -> Option<Pin<Strong<Slab>>> {
        while self.free == 0 {
            if flags.has_any(Alloc::NoVm) {
                return None;
            }

            #[allow(clippy::while_immutable_condition)] // TODO: Remove this.
            while self.max_pages != 0 && self.max_pages <= self.pages {
                todo!()
            }

            self.recurse += 1;
            let slab = self.alloc_slab(flags);
            self.recurse -= 1;

            if let Some(slab) = slab {
                // We cannot keep a strong reference to the slab here otherwise the consumer never
                // be able to drop it.
                let slab = unsafe { Pin::into_inner_unchecked(slab) };

                self.partial_slabs.push_front(Strong::as_ptr(&slab));

                return Some(unsafe { Pin::new_unchecked(slab) });
            }

            flags |= Alloc::NoVm;
        }

        if let Some(v) = self.partial_slabs.front().copied() {
            return Some(unsafe { Pin::new_unchecked(Strong::new(v.as_ptr())) });
        }

        todo!()
    }

    /// See `keg_alloc_slab` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13FBA0|
    fn alloc_slab(&mut self, flags: Alloc) -> Option<Pin<Strong<Slab>>> {
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
            let (mem, slab_flags) = (self.alloc)(self.vm, flags);

            if !mem.is_null() {
                // The Orbis also check if uk_flags does not contains UMA_ZONE_OFFPAGE, which seems
                // to be useless since we only be here when it does not contains UMA_ZONE_OFFPAGE.
                let hdr = unsafe { mem.byte_add(self.pgoff).cast::<SlabHdr>() };

                // TODO: I'm not confident about the memory layout here. The variables calculation
                // during keg construction is very complicated and I don't fully understand it. If
                // we encounter some memory corruptions then this is likely to be the root of
                // problem.
                unsafe { hdr.write(SlabHdr::new(slab_flags, mem, self.ipers)) };

                // Initialize free items. The offset calculation here should be optimized away.
                let (_, off) = Layout::new::<SlabHdr>()
                    .extend(Layout::new::<u8>())
                    .unwrap();
                let free = unsafe { hdr.byte_add(off).cast::<u8>() };

                for i in 0..self.ipers {
                    let item = (i + 1).try_into().unwrap();

                    unsafe { free.add(i).write(item) };
                }

                if self.init.is_some() {
                    todo!()
                }

                if self.flags.has_any(UmaFlags::Hash) {
                    todo!()
                }

                self.pages += self.ppera;
                self.free += self.ipers;

                // The Orbis do this before initialize the slab but we move it after initialization
                // instead.
                let slab = core::ptr::slice_from_raw_parts_mut(hdr, self.ipers) as *mut Slab;
                let slab = unsafe { Pin::new_unchecked(Strong::new(slab)) };

                if self.flags.has_any(UmaFlags::VToSlab) {
                    let mut next = mem as usize;

                    for _ in 0..self.ppera {
                        let p = unsafe { kaddr_to_phys(next) };
                        let p = self.vm.phys_to_page(p).unwrap(); // Orbis assume non-null.
                        let mut s = p.state.lock();

                        // The Orbis also set PG_SLAB to vm_page::flags here. AFAIK this flag only
                        // used to identify the vm_page::object, which mean we don't need this flag
                        // because our vm_page::object is a Rust enum.
                        s.object = Some(PageObj::Slab(slab.clone()));

                        drop(s);

                        next += PAGE_SIZE.get();
                    }
                }

                return Some(slab);
            }

            todo!()
        }
    }
}
