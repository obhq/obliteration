use super::{PhysMapping, RamBuilder};
use crate::Hypervisor;
use rustc_hash::FxHashMap;
use std::mem::transmute;
use std::num::NonZero;
use thiserror::Error;

impl<'a, H: Hypervisor> RamBuilder<'a, H> {
    pub(super) fn build_4k_page_tables(
        mut self,
        phys_vaddr: usize,
        phys_addrs: impl IntoIterator<Item = PhysMapping>,
    ) -> Result<usize, RamBuilderError> {
        // Allocate page-map level-4 table.
        //
        // See Page Translation and Protection section on AMD64 Architecture Programmer's Manual
        // Volume 2 for how paging work in long-mode.
        let (pml4t, page_table) = self
            .alloc_page_table()
            .ok_or(RamBuilderError::AllocPml4Table)?;

        // Setup page tables for allocated RAM.
        let page_size = self.vm_page_size;
        let mut cx = Context4K {
            pml4t,
            pdpt: FxHashMap::default(),
            pdt: FxHashMap::default(),
            pt: FxHashMap::default(),
        };

        for info in std::mem::take(&mut self.allocated) {
            let len = info.len.get().next_multiple_of(page_size.get());

            self.setup_4k_page_tables(&mut cx, info.vaddr, info.paddr, len)?;
        }

        // Setup page tables to map physical addresses.
        for phys in phys_addrs {
            let vaddr = phys_vaddr.checked_add(phys.addr).unwrap();
            let len = phys
                .len
                .get()
                .checked_next_multiple_of(page_size.get())
                .unwrap();

            self.setup_4k_page_tables(&mut cx, vaddr, phys.addr, len)?;
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

        for off in (0..len).step_by(4096) {
            use std::collections::hash_map::Entry;

            // Get page-directory pointer table.
            let addr = vaddr + off;
            let pml4o = (addr & 0xFF8000000000) >> 39;
            let pdpt = match cx.pml4t[pml4o] {
                0 => {
                    let (pdpt, addr) = self
                        .alloc_page_table()
                        .ok_or(RamBuilderError::AllocPdpTable)?;

                    Self::set_page_entry(&mut cx.pml4t[pml4o], addr);

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
                        .ok_or(RamBuilderError::AllocPdTable)?;

                    Self::set_page_entry(&mut pdpt[pdpo], addr);

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
                        .ok_or(RamBuilderError::AllocPageTable)?;

                    Self::set_page_entry(&mut pdt[pdo], addr);

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

            Self::set_page_entry(&mut pt[pto], paddr);
        }

        Ok(())
    }

    fn alloc_page_table(&mut self) -> Option<(&'a mut [usize; 512], usize)> {
        // Get address and length.
        let addr = self.next;
        let len = (512usize * 8)
            .checked_next_multiple_of(self.vm_page_size.get())
            .and_then(NonZero::new)
            .unwrap();

        // Page table on x86-64 always 4k aligned regardless page size being used.
        assert_eq!(addr % 4096, 0);

        // Allocate.
        let tab = self.hv.ram().slice(addr, len);

        if tab.is_null() {
            return None;
        }

        for i in 0..len.get() {
            unsafe { tab.add(i).write(0) };
        }

        self.next += len.get();

        Some((unsafe { transmute(tab) }, addr))
    }

    fn set_page_entry(entry: &mut usize, addr: usize) {
        assert_eq!(addr & 0x7FF0000000000000, 0);
        assert_eq!(addr & 0xFFF, 0);

        *entry = addr;
        *entry |= 0b01; // Present (P) Bit.
        *entry |= 0b10; // Read/Write (R/W) Bit.
    }
}

/// Context to build 4K page tables.
struct Context4K<'a> {
    pml4t: &'a mut [usize; 512],
    pdpt: FxHashMap<usize, &'a mut [usize; 512]>,
    pdt: FxHashMap<usize, &'a mut [usize; 512]>,
    pt: FxHashMap<usize, &'a mut [usize; 512]>,
}

/// Represents an error when [`RamBuilder::build_page_table()`] fails.
#[derive(Debug, Error)]
pub enum RamBuilderError {
    #[error("not enough RAM for page-map level-4 table")]
    AllocPml4Table,

    #[error("not enough RAM for page-directory pointer table")]
    AllocPdpTable,

    #[error("not enough RAM for page-directory table")]
    AllocPdTable,

    #[error("not enough RAM for page table")]
    AllocPageTable,

    #[error("duplicated virtual address {0:#x}")]
    DuplicatedVirtualAddr(usize),
}
