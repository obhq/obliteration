// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

use super::{LockedMem, RamError};
use crate::Hypervisor;
use std::num::NonZero;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

/// Struct to build the initial content of [Ram](super::Ram).
///
/// This struct also provide [`RamBuilder::build_page_table()`] to build page tables so you can run
/// the VM in a virtual address space from the beginning.
pub struct RamBuilder<'a, H> {
    hv: &'a H,
    next: usize,
    allocated: Vec<AllocInfo>,
}

impl<'a, H: Hypervisor> RamBuilder<'a, H> {
    /// `start_addr` is a physical address to start allocate a block of memory, not a start address
    /// of the RAM itself!
    ///
    /// # Panics
    /// If `start_addr` is not multiply by RAM block size.
    pub fn new(hv: &'a mut H, start_addr: usize) -> Self {
        assert_eq!(start_addr % hv.ram().block_size(), 0);

        Self {
            hv,
            next: start_addr,
            allocated: Vec::new(),
        }
    }

    /// Specify [`None`] for `vaddr` to use the same value as physical address (AKA identity
    /// mapping).
    ///
    /// The first item in the returned tuple is physical address of the returned [`LockedMem`],
    /// which always aligned to RAM block size. This imply the memory contained in the [`LockedMem`]
    /// also aligned to RAM block size.
    ///
    /// Returns [`RamError::InvalidAddr`] if available space is not enough for `len`.
    ///
    /// # Panics
    /// If `vaddr` is not multiply by VM page size.
    pub fn alloc(
        &mut self,
        vaddr: Option<usize>,
        len: NonZero<usize>,
    ) -> Result<(usize, LockedMem<'a>), RamError> {
        // Build alloc info.
        let paddr = self.next;
        let vaddr = vaddr.unwrap_or(paddr);
        let info = AllocInfo { paddr, vaddr, len };

        assert_eq!(vaddr % self.hv.ram().vm_page_size(), 0);

        // Alloc.
        let len = len
            .get()
            .checked_next_multiple_of(self.hv.ram().block_size().get())
            .ok_or(RamError::InvalidAddr)?;
        let len = unsafe { NonZero::new_unchecked(len) };
        let mut mem = self.hv.ram().alloc(paddr, len)?;

        self.allocated.push(info);
        self.next += len.get();

        // Fill with zeroes. We need this in case of the previous RamBuilder fails and the user
        // construct a new one.
        let ptr = mem.as_mut_ptr();

        for i in 0..len.get() {
            unsafe { ptr.add(i).write(0) };
        }

        Ok((paddr, mem))
    }
}

/// Contains information for external allocations.
struct AllocInfo {
    paddr: usize,
    vaddr: usize,
    len: NonZero<usize>,
}
