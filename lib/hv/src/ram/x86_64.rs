use super::RamBuilder;
use crate::{Hypervisor, LockedMem, RamError};
use rustc_hash::FxHashMap;
use std::num::NonZero;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

impl<'a, H: Hypervisor> RamBuilder<'a, H> {
    /// # Panics
    /// If any address in `devices` within RAM address or is not multiply by VM page size or its
    /// size cannot round to VM page size. The latter case only happen when the value is too large
    /// (e.g. 0xFFFFFFFFFFFFF000 for 4K page).
    pub fn build_page_table(
        self,
        devices: impl IntoIterator<Item = (usize, NonZero<usize>)>,
    ) -> Result<usize, RamBuilderError> {
        match self.hv.ram().vm_page_size().get() {
            0x1000 => self.build_4k_page_tables(devices),
            _ => todo!(),
        }
    }

    fn build_4k_page_tables(
        mut self,
        devices: impl IntoIterator<Item = (usize, NonZero<usize>)>,
    ) -> Result<usize, RamBuilderError> {
        // Allocate page-map level-4 table.
        //
        // See Page Translation and Protection section on AMD64 Architecture Programmer's Manual
        // Volume 2 for how paging work in long-mode.
        let (pml4t, page_table) = self
            .alloc_page_table()
            .map_err(RamBuilderError::AllocPml4Table)?;

        // Setup page tables for allocated blocks.
        let mut cx = Context4K {
            pml4t,
            pdpt: FxHashMap::default(),
            pdt: FxHashMap::default(),
            pt: FxHashMap::default(),
        };

        for info in std::mem::take(&mut self.allocated) {
            let len = info.len.get().next_multiple_of(4096);

            self.setup_4k_page_tables(&mut cx, info.vaddr, info.paddr, len)?;
        }

        // Setup page tables to map virtual devices. We use identity mapping for virtual devices.
        for (addr, len) in devices {
            let len = len.get().checked_next_multiple_of(4096).unwrap();

            assert!(addr >= self.hv.ram().len().get());

            self.setup_4k_page_tables(&mut cx, addr, addr, len)?;
        }

        Ok(page_table)
    }

    fn setup_4k_page_tables(
        &mut self,
        cx: &mut Context4K<'a>,
        vaddr: usize,
        paddr: usize,
        len: usize,
    ) -> Result<(), RamBuilderError> {
        assert_eq!(vaddr % 4096, 0);
        assert_eq!(paddr % 4096, 0);
        assert_eq!(len % 4096, 0);

        fn set_page_entry(entry: &mut usize, addr: usize) {
            assert_eq!(addr & 0x7FF0000000000000, 0);
            assert_eq!(addr & 0xFFF, 0);

            *entry = addr;
            *entry |= 0b01; // Present (P) Bit.
            *entry |= 0b10; // Read/Write (R/W) Bit.
        }

        for off in (0..len).step_by(4096) {
            use std::collections::hash_map::Entry;

            // Get page-directory pointer table.
            let addr = vaddr + off;
            let pml4o = (addr & 0xFF8000000000) >> 39;
            let pdpt = match cx.pml4t[pml4o] {
                0 => {
                    let (pdpt, addr) = self
                        .alloc_page_table()
                        .map_err(RamBuilderError::AllocPdpTable)?;

                    set_page_entry(&mut cx.pml4t[pml4o], addr);

                    match cx.pdpt.entry(addr) {
                        Entry::Occupied(_) => unreachable!(),
                        Entry::Vacant(e) => e.insert(pdpt),
                    }
                }
                v => cx.pdpt.get_mut(&(v & 0xFFFFFFFFFF000)).unwrap(),
            };

            // Get page-directory table.
            let pdpo = (addr & 0x7FC0000000) >> 30;
            let pdt = match pdpt[pdpo] {
                0 => {
                    let (pdt, addr) = self
                        .alloc_page_table()
                        .map_err(RamBuilderError::AllocPdTable)?;

                    set_page_entry(&mut pdpt[pdpo], addr);

                    match cx.pdt.entry(addr) {
                        Entry::Occupied(_) => unreachable!(),
                        Entry::Vacant(e) => e.insert(pdt),
                    }
                }
                v => cx.pdt.get_mut(&(v & 0xFFFFFFFFFF000)).unwrap(),
            };

            // Get page table.
            let pdo = (addr & 0x3FE00000) >> 21;
            let pt = match pdt[pdo] {
                0 => {
                    let (pt, addr) = self
                        .alloc_page_table()
                        .map_err(RamBuilderError::AllocPageTable)?;

                    set_page_entry(&mut pdt[pdo], addr);

                    match cx.pt.entry(addr) {
                        Entry::Occupied(_) => unreachable!(),
                        Entry::Vacant(e) => e.insert(pt),
                    }
                }
                v => cx.pt.get_mut(&(v & 0xFFFFFFFFFF000)).unwrap(),
            };

            // Set page table entry.
            let pto = (addr & 0x1FF000) >> 12;
            let paddr = paddr + off;

            if pt[pto] != 0 {
                return Err(RamBuilderError::DuplicatedVirtualAddr(addr));
            }

            set_page_entry(&mut pt[pto], paddr);
        }

        Ok(())
    }

    fn alloc_page_table(&mut self) -> Result<(PageTable<'a>, usize), RamError> {
        // Get address and length.
        let addr = self.next;
        let len = (512usize * 8)
            .checked_next_multiple_of(self.hv.ram().block_size().get())
            .and_then(NonZero::new)
            .unwrap();

        // Page table on x86-64 always 4k aligned regardless page size being used.
        assert_eq!(addr % 4096, 0);

        // Allocate.
        let mut tab = self.hv.ram().alloc(addr, len)?;

        self.next += len.get();

        // Fill with zeroes.
        let ptr = tab.as_mut_ptr();

        for i in 0..len.get() {
            unsafe { ptr.add(i).write(0) };
        }

        Ok((PageTable(tab), addr))
    }
}

/// Context to build 4K page tables.
struct Context4K<'a> {
    pml4t: PageTable<'a>,
    pdpt: FxHashMap<usize, PageTable<'a>>,
    pdt: FxHashMap<usize, PageTable<'a>>,
    pt: FxHashMap<usize, PageTable<'a>>,
}

/// Encapsulates a [`LockedMem`] containing a page table.
struct PageTable<'a>(LockedMem<'a>);

impl<'a> Deref for PageTable<'a> {
    type Target = [usize; 512];

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr().cast() }
    }
}

impl<'a> DerefMut for PageTable<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_mut_ptr().cast() }
    }
}

/// Represents an error when [`RamBuilder::build_page_table()`] fails
#[derive(Debug, Error)]
pub enum RamBuilderError {
    #[error("couldn't allocate page-map level-4 table")]
    AllocPml4Table(#[source] RamError),

    #[error("couldn't allocate page-directory pointer table")]
    AllocPdpTable(#[source] RamError),

    #[error("couldn't allocate page-directory table")]
    AllocPdTable(#[source] RamError),

    #[error("couldn't allocate page table")]
    AllocPageTable(#[source] RamError),

    #[error("duplicated virtual address {0:#x}")]
    DuplicatedVirtualAddr(usize),
}
