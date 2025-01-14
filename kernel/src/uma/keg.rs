use super::slab::RcFree;
use super::UmaFlags;
use crate::config::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use crate::uma::slab::{Free, SlabHdr};
use crate::uma::Uma;
use core::alloc::Layout;
use core::cmp::{max, min};
use core::num::NonZero;

/// Implementation of `uma_keg` structure.
pub struct UmaKeg {}

impl UmaKeg {
    /// See `keg_ctor` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x13CF40|
    pub(super) fn new(size: NonZero<usize>, align: usize, mut flags: UmaFlags) -> Self {
        if flags.has(UmaFlags::Vm) {
            todo!()
        }

        if flags.has(UmaFlags::ZInit) {
            todo!()
        }

        if flags.has(UmaFlags::Malloc | UmaFlags::RefCnt) {
            flags |= UmaFlags::VToSlab;
        }

        // Get uk_ppera.
        let ppera = if flags.has(UmaFlags::CacheSpread) {
            // Round size.
            let size = if (size.get() & align) == 0 {
                size.get()
            } else {
                (size.get() & !align) + align + 1
            };

            // Get uk_rsize.
            let align = align + 1;
            let rsize = if (size & align) == 0 {
                // TODO: What is this?
                size + align
            } else {
                size
            };

            // Get uk_ppera.
            let pages = (PAGE_SIZE.get() / align * rsize) >> PAGE_SHIFT;

            min(pages, (128 * 1024) / PAGE_SIZE)
        } else {
            // Check if item size exceed slab size.
            let min = Layout::new::<SlabHdr>();
            let (mut min, off) = if flags.has(UmaFlags::RefCnt) {
                min.extend(Layout::new::<RcFree>()).unwrap()
            } else {
                min.extend(Layout::new::<Free>()).unwrap()
            };

            min = min.pad_to_align();

            // Get UMA_FRITM_SZ and UMA_FRITMREF_SZ.
            let free_item = min.size() - off;
            let available = PAGE_SIZE.get() - min.size();

            // TODO: Not sure why we need space at least for 2 free item?
            if (size.get() + free_item) > available {
                // TODO: Set uk_ppera, uk_ipers and uk_rsize.
                if !flags.has(UmaFlags::Internal) {
                    flags |= UmaFlags::Offpage;

                    if !flags.has(UmaFlags::VToSlab) {
                        flags |= UmaFlags::Hash;
                    }
                }

                // Get uk_ppera.
                let mut ppera = size.get() >> PAGE_SHIFT;

                if size.get() > (size.get() & !PAGE_MASK.get()) {
                    ppera += 1;
                }

                ppera
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
                let ipers = available / (rsize + free_item);

                // TODO: Verify if this valid for PAGE_SIZE < 0x4000.
                if !flags.has(UmaFlags::Internal | UmaFlags::Cacheonly)
                    && (available % (rsize + free_item)) >= Uma::MAX_WASTE.get()
                    && (PAGE_SIZE.get() / rsize) > ipers
                {
                    todo!()
                }

                1
            }
        };

        if flags.has(UmaFlags::Offpage) {
            if flags.has(UmaFlags::RefCnt) {
                // TODO: Set uk_slabzone to slabrefzone.
            } else {
                // TODO: Set uk_slabzone to slabzone.
            }
        }

        if ppera == 1 {
            // TODO: Set uk_allocf and uk_freef.
        }

        if flags.has(UmaFlags::MtxClass) {
            todo!()
        }

        if !flags.has(UmaFlags::Offpage) {
            todo!()
        }

        todo!()
    }
}
