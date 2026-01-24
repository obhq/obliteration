use super::{PhysMapping, RamBuilder};
use crate::Hypervisor;
use rustc_hash::FxHashMap;
use std::mem::transmute;
use thiserror::Error;

impl<'a, H: Hypervisor> RamBuilder<'a, H> {
    pub(super) fn build_4k_page_tables(
        self,
        phys_vaddr: usize,
        phys_addrs: impl IntoIterator<Item = PhysMapping>,
    ) -> Result<usize, RamBuilderError> {
        todo!()
    }

    pub(super) fn build_16k_page_tables(
        mut self,
        phys_vaddr: usize,
        phys_addrs: impl IntoIterator<Item = PhysMapping>,
    ) -> Result<usize, RamBuilderError> {
        // Allocate page table level 0.
        let page_table = self.next;
        let len = (8usize * 32)
            .next_multiple_of(self.vm_page_size.get())
            .try_into()
            .unwrap();
        let l0t = self.hv.ram().slice(page_table, len);
        let l0t = match l0t.is_null() {
            true => return Err(RamBuilderError::AllocPageTableLevel0),
            false => {
                for i in 0..len.get() {
                    unsafe { l0t.add(i).write(0) };
                }

                unsafe { transmute(l0t) }
            }
        };

        self.next += len.get();

        // Setup page tables for allocated RAM.
        let mut cx = Context16K {
            l0t,
            l1t: FxHashMap::default(),
            l2t: FxHashMap::default(),
            l3t: FxHashMap::default(),
        };

        for info in std::mem::take(&mut self.allocated) {
            let len = info.len.get().next_multiple_of(0x4000);

            self.setup_16k_page_tables(&mut cx, info.vaddr, info.paddr, len, info.attr)?;
        }

        // Setup page tables to map physical addresses.
        for phys in phys_addrs {
            let vaddr = phys_vaddr.checked_add(phys.addr).unwrap();
            let len = phys.len.get().checked_next_multiple_of(0x4000).unwrap();

            self.setup_16k_page_tables(&mut cx, vaddr, phys.addr, len, phys.attr)?;
        }

        Ok(page_table)
    }

    fn setup_16k_page_tables(
        &mut self,
        cx: &mut Context16K<'a>,
        vaddr: usize,
        paddr: usize,
        len: usize,
        attr: u8,
    ) -> Result<(), RamBuilderError> {
        let attr = usize::from(attr);

        assert_eq!(vaddr % 0x4000, 0);
        assert_eq!(paddr % 0x4000, 0);
        assert_eq!(len % 0x4000, 0);
        assert_eq!(attr & 0b11111000, 0);

        fn set_table_descriptor(entry: &mut usize, addr: usize) {
            assert_eq!(addr & 0xFFFF000000003FFF, 0);

            *entry = addr;
            *entry |= 0b11; // Valid + Table descriptor/Page descriptor
            *entry |= 1 << 10; // AF
        }

        for off in (0..len).step_by(0x4000) {
            use std::collections::hash_map::Entry;

            // Get level 1 table.
            let addr = vaddr + off;
            let l0o = (addr & 0x800000000000) >> 47;
            let l1t = match cx.l0t[l0o] {
                0 => {
                    let (l1t, addr) = self
                        .alloc_16k_page_table()
                        .ok_or(RamBuilderError::AllocPageTableLevel1)?;

                    set_table_descriptor(&mut cx.l0t[l0o], addr);

                    match cx.l1t.entry(addr) {
                        Entry::Occupied(_) => unreachable!(),
                        Entry::Vacant(e) => e.insert(l1t),
                    }
                }
                v => cx.l1t.get_mut(&(v & 0xFFFFFFFFC000)).unwrap(),
            };

            // Get level 2 table.
            let l1o = (addr & 0x7FF000000000) >> 36;
            let l2t = match l1t[l1o] {
                0 => {
                    let (l2t, addr) = self
                        .alloc_16k_page_table()
                        .ok_or(RamBuilderError::AllocPageTableLevel2)?;

                    set_table_descriptor(&mut l1t[l1o], addr);

                    match cx.l2t.entry(addr) {
                        Entry::Occupied(_) => unreachable!(),
                        Entry::Vacant(e) => e.insert(l2t),
                    }
                }
                v => cx.l2t.get_mut(&(v & 0xFFFFFFFFC000)).unwrap(),
            };

            // Get level 3 table.
            let l2o = (addr & 0xFFE000000) >> 25;
            let l3t = match l2t[l2o] {
                0 => {
                    let (l3t, addr) = self
                        .alloc_16k_page_table()
                        .ok_or(RamBuilderError::AllocPageTableLevel3)?;

                    set_table_descriptor(&mut l2t[l2o], addr);

                    match cx.l3t.entry(addr) {
                        Entry::Occupied(_) => unreachable!(),
                        Entry::Vacant(e) => e.insert(l3t),
                    }
                }
                v => cx.l3t.get_mut(&(v & 0xFFFFFFFFC000)).unwrap(),
            };

            // Set page descriptor.
            let l3o = (addr & 0x1FFC000) >> 14;
            let paddr = paddr + off;
            let mut desc = paddr;

            assert_eq!(paddr & 0xFFFF000000003FFF, 0);

            if l3t[l3o] != 0 {
                return Err(RamBuilderError::DuplicatedVirtualAddr(addr));
            }

            desc |= 0b11; // Valid descriptor + Page descriptor
            desc |= attr << 2; // AttrIndx[2:0]
            desc |= 0b00 << 6; // AP[2:1]
            desc |= 0b11 << 8; // Inner Shareable
            desc |= 1 << 10; // AF

            l3t[l3o] = desc;
        }

        Ok(())
    }

    fn alloc_16k_page_table(&mut self) -> Option<(&'a mut [usize; 2048], usize)> {
        // Get address and length.
        let addr = self.next;
        let len = (8usize * 2048)
            .next_multiple_of(self.vm_page_size.get())
            .try_into()
            .unwrap();

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
}

/// Context to build 16K page tables.
struct Context16K<'a> {
    l0t: &'a mut [usize; 32],
    l1t: FxHashMap<usize, &'a mut [usize; 2048]>,
    l2t: FxHashMap<usize, &'a mut [usize; 2048]>,
    l3t: FxHashMap<usize, &'a mut [usize; 2048]>,
}

/// Represents an error when [`RamBuilder::build_page_table()`] fails.
#[derive(Debug, Error)]
pub enum RamBuilderError {
    #[error("not enough RAM for page table level 0")]
    AllocPageTableLevel0,

    #[error("not enough RAM for page table level 1")]
    AllocPageTableLevel1,

    #[error("not enough RAM for page table level 2")]
    AllocPageTableLevel2,

    #[error("not enough RAM for page table level 3")]
    AllocPageTableLevel3,

    #[error("duplicated virtual address {0:#x}")]
    DuplicatedVirtualAddr(usize),
}
