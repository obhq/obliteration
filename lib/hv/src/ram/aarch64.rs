use super::RamBuilder;
use crate::{Hypervisor, RamError};
use std::num::NonZero;
use thiserror::Error;

impl<H: Hypervisor> RamBuilder<'_, H> {
    const MA_DEV_NG_NR_NE: u8 = 0; // MEMORY_ATTRS[0]
    const MA_NOR: u8 = 1; // MEMORY_ATTRS[1]

    pub fn build_page_table(
        mut self,
        devices: impl IntoIterator<Item = (usize, NonZero<usize>)>,
    ) -> Result<usize, RamBuilderError> {
        // Setup page tables.
        let page_table = match self.hv.ram().vm_page_size().get() {
            0x4000 => self.build_16k_page_tables(devices)?,
            _ => todo!(),
        };

        // Flush modified memory.
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);

        Ok(page_table)
    }

    fn build_16k_page_tables(&mut self, devices: &DeviceTree) -> Result<usize, RamBuilderError> {
        // Allocate page table level 0.
        let page_table = self.next;
        let len = self.ram.block_size();
        let l0t: &mut [usize; 32] = match self.ram.alloc(page_table, len) {
            Ok(mut v) => unsafe { &mut *v.as_mut_ptr().cast() },
            Err(e) => return Err(RamBuilderError::AllocPageTableLevel0(e)),
        };

        self.next += len.get();

        // Map virtual devices. We use identity mapping for virtual devices.
        let mut dev_end = 0;

        for (addr, dev) in devices.all() {
            let len = dev.len().get();
            self.setup_16k_page_tables(l0t, addr, addr, len, Self::MA_DEV_NG_NR_NE)?;
            dev_end = addr + len;
        }

        // Setup page tables to map virtual address 0xffffffff82200000 to the kernel.
        // TODO: Implement ASLR.
        let mut vaddr = 0xffffffff82200000;
        let kern_vaddr = vaddr;
        let (kern_paddr, kern_len) = self
            .kern
            .take()
            .map(|v| (v.start, v.end - v.start))
            .unwrap();

        assert!(vaddr >= dev_end);

        self.setup_16k_page_tables(l0t, vaddr, kern_paddr, kern_len, Self::MA_NOR)?;

        vaddr += kern_len;

        // Setup page tables to map stack.
        let stack_vaddr = vaddr;
        let (paddr, stack_len) = self
            .stack
            .take()
            .map(|v| (v.start, v.end - v.start))
            .unwrap();

        self.setup_16k_page_tables(l0t, vaddr, paddr, stack_len, Self::MA_NOR)?;

        vaddr += stack_len;

        // Setup page tables to map arguments.
        let args = self.args.take().unwrap();
        let ram = args.ram;
        let env_vaddr = vaddr + args.env;
        let conf_vaddr = vaddr + args.conf;

        self.setup_16k_page_tables(l0t, vaddr, ram.start, ram.end - ram.start, Self::MA_NOR)?;

        Ok(page_table)
    }

    fn setup_16k_page_tables(
        &mut self,
        l0t: &mut [usize; 32],
        vaddr: usize,
        paddr: usize,
        len: usize,
        attr: u8,
    ) -> Result<(), RamBuilderError> {
        let attr: usize = attr.into();
        let ram = self.ram.host_addr().cast_mut(); // TODO: Make this safer.

        assert_eq!(len % 0x4000, 0);
        assert_eq!(attr & 0b11111000, 0);

        fn set_table_descriptor(entry: &mut usize, addr: usize) {
            assert_eq!(addr & 0xFFFF000000003FFF, 0);

            *entry = addr;
            *entry |= 0b11; // Valid + Table descriptor/Page descriptor
            *entry |= 1 << 10; // AF
        }

        for off in (0..len).step_by(0x4000) {
            // Get level 1 table.
            let addr = vaddr + off;
            let l0o = (addr & 0x800000000000) >> 47;
            let l1t = match l0t[l0o] {
                0 => {
                    let (l1t, addr) = self
                        .alloc_16k_page_table()
                        .map_err(RamBuilderError::AllocPageTableLevel1)?;

                    set_table_descriptor(&mut l0t[l0o], addr);

                    unsafe { &mut *l1t }
                }
                v => unsafe { &mut *ram.add(v & 0xFFFFFFFFC000).cast() },
            };

            // Get level 2 table.
            let l1o = (addr & 0x7FF000000000) >> 36;
            let l2t = match l1t[l1o] {
                0 => {
                    let (l2t, addr) = self
                        .alloc_16k_page_table()
                        .map_err(RamBuilderError::AllocPageTableLevel2)?;

                    set_table_descriptor(&mut l1t[l1o], addr);

                    unsafe { &mut *l2t }
                }
                v => unsafe { &mut *ram.add(v & 0xFFFFFFFFC000).cast() },
            };

            // Get level 3 table.
            let l2o = (addr & 0xFFE000000) >> 25;
            let l3t = match l2t[l2o] {
                0 => {
                    let (l3t, addr) = self
                        .alloc_16k_page_table()
                        .map_err(RamBuilderError::AllocPageTableLevel3)?;

                    set_table_descriptor(&mut l2t[l2o], addr);

                    unsafe { &mut *l3t }
                }
                v => unsafe { &mut *ram.add(v & 0xFFFFFFFFC000).cast() },
            };

            // Set page descriptor.
            let l3o = (addr & 0x1FFC000) >> 14;
            let addr = paddr + off;
            let mut desc = addr;

            assert_eq!(addr & 0xFFFF000000003FFF, 0);
            assert_eq!(l3t[l3o], 0);

            desc |= 0b11; // Valid descriptor + Page descriptor
            desc |= attr << 2; // AttrIndx[2:0]
            desc |= 0b00 << 6; // AP[2:1]
            desc |= 0b11 << 8; // Inner Shareable
            desc |= 1 << 10; // AF

            l3t[l3o] = desc;
        }

        Ok(())
    }

    fn alloc_16k_page_table(&mut self) -> Result<(*mut [usize; 2048], usize), RamError> {
        // Get address and length.
        let addr = self.next;
        let len = (2048usize * 8)
            .checked_next_multiple_of(self.ram.block_size().get())
            .and_then(NonZero::new)
            .unwrap();

        // Allocate.
        let tab = self
            .ram
            .alloc(addr, len)
            .map(|mut v| v.as_mut_ptr().cast())?;

        self.next += len.get();

        Ok((tab, addr))
    }
}

/// Represents an error when [`RamBuilder::build_page_table()`] fails
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
}
