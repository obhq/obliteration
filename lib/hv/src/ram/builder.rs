// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

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
    vm_page_size: NonZero<usize>,
    next: usize,
    allocated: Vec<AllocInfo>,
}

impl<'a, H: Hypervisor> RamBuilder<'a, H> {
    /// This function need a mutable borrow to the hypervisor to make sure no any vCPU is currently
    /// running.
    ///
    /// # Panics
    /// If `vm_page_size` is not power of two.
    pub fn new(hv: &'a mut H, vm_page_size: NonZero<usize>) -> Self {
        assert!(vm_page_size.is_power_of_two());

        Self {
            hv,
            vm_page_size,
            next: 0,
            allocated: Vec::new(),
        }
    }

    pub fn next_addr(&self) -> usize {
        self.next
    }

    /// Allocate a single VM page.
    ///
    /// Specify [None] for `vaddr` to use the same value as physical address (AKA identity mapping).
    ///
    /// The first item in the returned tuple is a physical address of the returned slice, which
    /// always aligned to VM page size.
    ///
    /// Returns [None] if available space is not enough for `len`.
    ///
    /// # Panics
    /// If `vaddr` is not multiply by VM page size.
    pub fn alloc(
        &mut self,
        vaddr: Option<usize>,
        len: NonZero<usize>,
        #[cfg(target_arch = "aarch64")] attr: u8,
    ) -> Option<(usize, &'a mut [u8])> {
        // Build alloc info.
        let paddr = self.next;
        let vaddr = vaddr.unwrap_or(paddr);
        let info = AllocInfo {
            paddr,
            vaddr,
            len,
            #[cfg(target_arch = "aarch64")]
            attr,
        };

        assert_eq!(vaddr % self.vm_page_size, 0);

        // Get size to allocate. We always round to VM page size
        // Alloc.
        let len = len
            .get()
            .checked_next_multiple_of(self.vm_page_size.get())?;
        let len = unsafe { NonZero::new_unchecked(len) };
        let ptr = self.hv.ram().slice(paddr, len);

        if ptr.is_null() {
            return None;
        }

        self.allocated.push(info);
        self.next += len.get();

        // Fill with zeroes. We need this in case of the previous RamBuilder fails and the user
        // construct a new one.
        for i in 0..len.get() {
            unsafe { ptr.add(i).write(0) };
        }

        // Now it is safe to create a slice.
        let mem = unsafe { std::slice::from_raw_parts_mut(ptr, len.get()) };

        Some((paddr, mem))
    }

    /// # Panics
    /// - If `phys_vaddr` addition with [PhysMapping::addr] from `phys_addrs` is overflow or not
    ///   multiply by VM page size.
    /// - If [PhysMapping::len] in `phys_addrs` size cannot round to VM page size. The can only
    ///   happen when the value is too large (e.g. 0xFFFFFFFFFFFFF000 for 4K page).
    pub fn build_page_table(
        self,
        phys_vaddr: usize,
        phys_addrs: impl IntoIterator<Item = PhysMapping>,
    ) -> Result<usize, RamBuilderError> {
        match self.vm_page_size.get() {
            0x1000 => self.build_4k_page_tables(phys_vaddr, phys_addrs),
            #[cfg(target_arch = "aarch64")]
            0x4000 => self.build_16k_page_tables(phys_vaddr, phys_addrs),
            #[cfg(target_arch = "x86_64")]
            0x4000 => self.build_4k_page_tables(phys_vaddr, phys_addrs),
            _ => todo!(),
        }
    }
}

/// Contains information for an allocation in a virtual address space.
pub struct AllocInfo {
    pub paddr: usize,
    pub vaddr: usize,
    pub len: NonZero<usize>,
    #[cfg(target_arch = "aarch64")]
    pub attr: u8,
}

/// Contains information how to map a range of physical address to virtual address.
pub struct PhysMapping {
    pub addr: usize,
    pub len: NonZero<usize>,
    #[cfg(target_arch = "aarch64")]
    pub attr: u8,
}
