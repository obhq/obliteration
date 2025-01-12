use super::slab::RcFree;
use super::UmaFlags;
use crate::config::PAGE_SIZE;
use crate::uma::slab::{Free, SlabHdr};
use core::alloc::Layout;
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
    pub(super) fn new(size: NonZero<usize>, _: usize, mut flags: UmaFlags) -> Self {
        if flags.has(UmaFlags::Vm) {
            todo!()
        }

        if flags.has(UmaFlags::ZInit) {
            todo!()
        }

        if flags.has(UmaFlags::Malloc | UmaFlags::RefCnt) {
            flags |= UmaFlags::VToSlab;
        }

        if flags.has(UmaFlags::CacheSpread) {
            todo!()
        } else {
            // Check if item size exceed slab size.
            let min = Layout::new::<SlabHdr>();
            let (mut min, off) = if flags.has(UmaFlags::RefCnt) {
                min.extend(Layout::new::<RcFree>()).unwrap()
            } else {
                min.extend(Layout::new::<Free>()).unwrap()
            };

            min = min.pad_to_align();

            // TODO: Not sure why we need space at least for 2 free item?
            if (size.get() + (min.size() - off)) > (PAGE_SIZE.get() - min.size()) {
                todo!()
            } else {
                if !flags.has(UmaFlags::Internal) {
                    flags |= UmaFlags::Offpage;

                    if !flags.has(UmaFlags::VToSlab) {
                        flags |= UmaFlags::Hash;
                    }
                }
            }
        }

        if flags.has(UmaFlags::Offpage) {
            if flags.has(UmaFlags::RefCnt) {
                // TODO: Set uk_slabzone to slabrefzone.
            } else {
                // TODO: Set uk_slabzone to slabzone.
            }
        }

        todo!()
    }
}
