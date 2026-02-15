use super::arch::small_alloc;
use super::slab::{Free, RcFree, Slab};
use super::{Alloc, Uma, UmaFlags, UmaZone};
use crate::config::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use crate::vm::Vm;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::cmp::{max, min};
use core::mem::MaybeUninit;
use core::num::NonZero;

/// Implementation of `uma_keg` structure.
pub struct UmaKeg {
    vm: Arc<Vm>,
    size: NonZero<usize>,             // uk_size
    pgoff: usize,                     // uk_pgoff
    ppera: usize,                     // uk_ppera
    ipers: usize,                     // uk_ipers
    alloc: fn(&Vm, Alloc) -> *mut u8, // uk_allocf
    init: Option<fn()>,               // uk_init
    max_pages: usize,                 // uk_maxpages
    pages: usize,                     // uk_pages
    free: usize,                      // uk_free
    recurse: u32,                     // uk_recurse
    flags: UmaFlags,                  // uk_flags
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

        if flags.has_any(UmaFlags::Malloc | UmaFlags::RefCnt) {
            flags |= UmaFlags::VToSlab;
        }

        // Get header layout.
        let hdr = Layout::new::<Slab<()>>();
        let (mut hdr, off) = if flags.has_any(UmaFlags::RefCnt) {
            hdr.extend(Layout::new::<RcFree>()).unwrap()
        } else {
            hdr.extend(Layout::new::<Free>()).unwrap()
        };

        hdr = hdr.pad_to_align();

        // Get UMA_FRITM_SZ and UMA_FRITMREF_SZ.
        let free_item = hdr.size() - off;
        let available = PAGE_SIZE.get() - hdr.size();

        // Get uk_ppera and uk_ipers.
        let (ppera, ipers) = if flags.has_any(UmaFlags::CacheSpread) {
            // Round size.
            let rsize = if (size.get() & align) == 0 {
                size.get()
            } else {
                (size.get() & !align) + align + 1
            };

            // Get uk_rsize.
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

            // Get uk_ipers.
            let ipers = (ppera * PAGE_SIZE.get() + (rsize - size.get())) / rsize;

            (ppera, ipers)
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

                (ppera, 1)
            } else {
                // Get uk_rsize.
                let rsize = max(size, Uma::SMALLEST_UNIT);
                let rsize = if (align & rsize.get()) == 0 {
                    rsize.get()
                } else {
                    // Size is not multiple of alignment, align up.
                    align + 1 + (!align & rsize.get())
                };

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

                (1, ipers)
            }
        };

        if flags.has_any(UmaFlags::Offpage) {
            if flags.has_any(UmaFlags::RefCnt) {
                // TODO: Set uk_slabzone to slabrefzone.
            } else {
                // TODO: Set uk_slabzone to slabzone.
            }
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
            pgoff,
            ppera,
            ipers,
            alloc,
            init,
            max_pages: 0,
            pages: 0,
            free: 0,
            recurse: 0,
            flags,
        }
    }

    pub fn size(&self) -> NonZero<usize> {
        self.size
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

    /// See `keg_fetch_slab` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x141E20|
    pub fn fetch_slab(&mut self, _: &UmaZone, flags: Alloc) -> Option<()> {
        while self.free == 0 {
            if flags.has_any(Alloc::NoVm) {
                return None;
            }

            #[allow(clippy::while_immutable_condition)] // TODO: Remove this.
            while self.max_pages != 0 && self.max_pages <= self.pages {
                todo!()
            }

            self.recurse += 1;
            self.alloc_slab(flags);
            self.recurse -= 1;

            todo!()
        }

        todo!()
    }

    /// See `keg_alloc_slab` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13FBA0|
    fn alloc_slab(&mut self, flags: Alloc) -> *mut Slab<()> {
        let mut slab: *mut Slab<()>;

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
            slab = (self.alloc)(&self.vm, flags).cast();

            if !slab.is_null() {
                // The Orbis also check if uk_flags does not contains UMA_ZONE_OFFPAGE, which seems
                // to be useless since we only be here when it does not contains UMA_ZONE_OFFPAGE.
                slab = unsafe { slab.byte_add(self.pgoff) };

                if self.flags.has_any(UmaFlags::VToSlab) && self.ppera != 0 {
                    todo!()
                }

                // TODO: Populate slab.
                if self.flags.has_any(UmaFlags::RefCnt) {
                    todo!()
                } else if self.ipers == 0 {
                    todo!()
                } else {
                    let slab = core::ptr::slice_from_raw_parts_mut(slab, self.ipers);
                    let slab = slab as *mut Slab<[MaybeUninit<Free>]>;

                    for (i, f) in unsafe { (*slab).free.iter_mut().enumerate() } {
                        f.write(Free {
                            item: (i + 1).try_into().unwrap(),
                        });
                    }
                }

                if self.init.is_some() {
                    todo!()
                }

                if self.flags.has_any(UmaFlags::Hash) {
                    todo!()
                }

                self.pages += self.ppera;
                self.free += self.ipers;

                return slab;
            }

            todo!()
        }
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
