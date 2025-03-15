use super::{AllocInfo, RamBuilder};
use crate::{Hypervisor, LockedMem, RamError};
use rustc_hash::FxHashMap;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

impl<'a, H: Hypervisor> RamBuilder<'a, H> {
    pub(super) fn build_4k_page_tables(
        self,
        _: impl IntoIterator<Item = AllocInfo>,
    ) -> Result<usize, RamBuilderError> {
        todo!()
    }

    pub(super) fn build_16k_page_tables(
        mut self,
        devices: impl IntoIterator<Item = AllocInfo>,
    ) -> Result<usize, RamBuilderError> {
        // Allocate page table level 0.
        let page_table = self.next;
        let len = (8usize * 32)
            .next_multiple_of(self.hv.ram().block_size().get())
            .try_into()
            .unwrap();
        let l0t = match self.hv.ram().alloc(page_table, len) {
            Ok(mut v) => {
                v.fill(0);
                Root16K(v)
            }
            Err(e) => return Err(RamBuilderError::AllocPageTableLevel0(e)),
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

        // Setup page tables to map virtual devices.
        for dev in devices {
            let len = dev.len.get().checked_next_multiple_of(0x4000).unwrap();

            assert!(dev.paddr >= self.hv.ram().len().get());

            self.setup_16k_page_tables(&mut cx, dev.vaddr, dev.paddr, len, dev.attr)?;
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
                        .map_err(RamBuilderError::AllocPageTableLevel1)?;

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
                        .map_err(RamBuilderError::AllocPageTableLevel2)?;

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
                        .map_err(RamBuilderError::AllocPageTableLevel3)?;

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

    fn alloc_16k_page_table(&mut self) -> Result<(Table16K<'a>, usize), RamError> {
        // Get address and length.
        let addr = self.next;
        let len = (8usize * 2048)
            .next_multiple_of(self.hv.ram().block_size().get())
            .try_into()
            .unwrap();

        // Allocate.
        let mut tab = self.hv.ram().alloc(addr, len)?;

        tab.fill(0);

        self.next += len.get();

        Ok((Table16K(tab), addr))
    }
}

/// Context to build 16K page tables.
struct Context16K<'a> {
    l0t: Root16K<'a>,
    l1t: FxHashMap<usize, Table16K<'a>>,
    l2t: FxHashMap<usize, Table16K<'a>>,
    l3t: FxHashMap<usize, Table16K<'a>>,
}

/// Encapsulates a [`LockedMem`] containing page table level 0 for 16K page.
struct Root16K<'a>(LockedMem<'a>);

impl Deref for Root16K<'_> {
    type Target = [usize; 32];

    fn deref(&self) -> &Self::Target {
        // SAFETY: RamBuilder require a mutable borrow to the hypervivor, which mean no any vCPU is
        // running for sure.
        unsafe { &*self.0.as_ptr().cast() }
    }
}

impl DerefMut for Root16K<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: RamBuilder require a mutable borrow to the hypervivor, which mean no any vCPU is
        // running for sure.
        unsafe { &mut *self.0.as_mut_ptr().cast() }
    }
}

/// Encapsulates a [`LockedMem`] containing page table level 1/2/3 for 16K page.
struct Table16K<'a>(LockedMem<'a>);

impl Deref for Table16K<'_> {
    type Target = [usize; 2048];

    fn deref(&self) -> &Self::Target {
        // SAFETY: RamBuilder require a mutable borrow to the hypervivor, which mean no any vCPU is
        // running for sure.
        unsafe { &*self.0.as_ptr().cast() }
    }
}

impl DerefMut for Table16K<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: RamBuilder require a mutable borrow to the hypervivor, which mean no any vCPU is
        // running for sure.
        unsafe { &mut *self.0.as_mut_ptr().cast() }
    }
}

/// Represents an error when [`RamBuilder::build_page_table()`] fails.
#[derive(Debug, Error)]
pub enum RamBuilderError {
    #[error("couldn't allocate page table level 0")]
    AllocPageTableLevel0(#[source] RamError),

    #[error("couldn't allocate page table level 1")]
    AllocPageTableLevel1(#[source] RamError),

    #[error("couldn't allocate page table level 2")]
    AllocPageTableLevel2(#[source] RamError),

    #[error("couldn't allocate page table level 3")]
    AllocPageTableLevel3(#[source] RamError),

    #[error("duplicated virtual address {0:#x}")]
    DuplicatedVirtualAddr(usize),
}
