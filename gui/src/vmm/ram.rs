// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Ram;
use crate::vmm::hw::DeviceTree;
use crate::vmm::kernel::ProgramHeader;
use config::{BootEnv, Config};
use hv::{CpuFeats, LockedMem, RamError, RamMapper};
use std::num::NonZero;
use std::ops::Range;
use thiserror::Error;

/// Struct to build [`Ram`].
pub struct RamBuilder<'a, M: RamMapper> {
    ram: &'a mut Ram<M>,
    next: usize,
    kern: Option<Range<usize>>,
    stack: Option<Range<usize>>,
    args: Option<KernelArgs>,
}

impl<'a, M: RamMapper> RamBuilder<'a, M> {
    pub fn new(ram: &'a mut Ram<M>) -> Self {
        Self {
            ram,
            next: 0,
            kern: None,
            stack: None,
            args: None,
        }
    }

    /// # Panics
    /// - If `len` is not multiplied by block size.
    /// - If called a second time.
    pub fn alloc_kernel(&mut self, len: NonZero<usize>) -> Result<LockedMem<M>, RamError> {
        assert!(self.kern.is_none());

        let addr = self.next;
        let mem = self.ram.alloc(addr, len)?;

        self.kern = Some(addr..(addr + len.get()));
        self.next += len.get();

        Ok(mem)
    }

    /// # Panics
    /// - If `len` is not multiplied by block size.
    /// - If called a second time.
    pub fn alloc_stack(&mut self, len: NonZero<usize>) -> Result<(), RamError> {
        assert!(self.stack.is_none());

        let addr = self.next;

        self.ram.alloc(addr, len)?;

        self.stack = Some(addr..(addr + len.get()));
        self.next += len.get();

        Ok(())
    }

    /// # Panics
    /// If called a second time.
    pub fn alloc_args(&mut self, env: BootEnv, conf: Config) -> Result<(), RamError> {
        assert!(self.args.is_none());
        assert!(align_of::<BootEnv>() <= self.ram.block_size().get());

        // Allocate RAM for all arguments.
        let addr = self.next;
        let len = size_of::<BootEnv>()
            .checked_next_multiple_of(align_of::<Config>())
            .and_then(|off| off.checked_add(size_of::<Config>()))
            .and_then(|len| len.checked_next_multiple_of(self.ram.block_size().get()))
            .and_then(NonZero::new)
            .unwrap();

        let args = self.ram.alloc(addr, len)?;
        let mut w = ArgsWriter { mem: args, next: 0 };

        // Write arguments.
        let env = w.write(env);
        let conf = w.write(conf);

        // Write conf.
        self.args = Some(KernelArgs {
            ram: addr..(addr + len.get()),
            env,
            conf,
        });

        self.next += len.get();

        Ok(())
    }

    /// # Safety
    /// [`RamMap::kern_paddr`] and [`RamMap::kern_len`] must be valid.
    unsafe fn relocate_kernel(
        &mut self,
        map: &RamMap,
        dynamic: ProgramHeader,
        ty: usize,
    ) -> Result<(), RamBuilderError> {
        // Check if PT_DYNAMIC valid.
        let p_vaddr = dynamic.p_vaddr;
        let p_memsz = dynamic.p_memsz;

        if p_memsz % 16 != 0 {
            return Err(RamBuilderError::InvalidDynamicLinking);
        }

        // Get PT_DYNAMIC.
        let paddr = map.kern_paddr;
        let mut kern = self
            .ram
            .lock(paddr, map.kern_len.try_into().unwrap())
            .unwrap();
        let kern = unsafe { std::slice::from_raw_parts_mut(kern.as_mut_ptr(), kern.len().get()) };
        let dynamic = p_vaddr
            .checked_add(p_memsz)
            .and_then(|end| kern.get(p_vaddr..end))
            .ok_or(RamBuilderError::InvalidDynamicLinking)?;

        // Parse PT_DYNAMIC.
        let mut rela = None;
        let mut relasz = None;

        for entry in dynamic.chunks_exact(16) {
            let tag = usize::from_ne_bytes(entry[..8].try_into().unwrap());
            let val = usize::from_ne_bytes(entry[8..].try_into().unwrap());

            match tag {
                0 => break,              // DT_NULL
                7 => rela = Some(val),   // DT_RELA
                8 => relasz = Some(val), // DT_RELASZ
                _ => {}
            }
        }

        // Check DT_RELA and DT_RELASZ.
        let (relocs, len) = match (rela, relasz) {
            (None, None) => return Ok(()),
            (Some(rela), Some(relasz)) => (rela, relasz),
            _ => return Err(RamBuilderError::InvalidDynamicLinking),
        };

        // Check if size valid.
        if (len % 24) != 0 || relocs.checked_add(len).is_none_or(|end| end > kern.len()) {
            return Err(RamBuilderError::InvalidDynamicLinking);
        }

        // Apply relocations.
        for off in (0..len).step_by(24).map(|v| relocs + v) {
            let r_offset = usize::from_ne_bytes(kern[off..(off + 8)].try_into().unwrap());
            let r_info = usize::from_ne_bytes(kern[(off + 8)..(off + 16)].try_into().unwrap());
            let r_addend = isize::from_ne_bytes(kern[(off + 16)..(off + 24)].try_into().unwrap());

            match r_info & 0xffffffff {
                // R_<ARCH>_NONE
                0 => break,
                // R_<ARCH>_RELATIVE
                v if v == ty => {
                    let dst = r_offset
                        .checked_add(8)
                        .and_then(|end| kern.get_mut(r_offset..end))
                        .ok_or(RamBuilderError::InvalidDynamicLinking)?;
                    let val = map.kern_vaddr.wrapping_add_signed(r_addend);

                    unsafe { core::ptr::write_unaligned(dst.as_mut_ptr().cast(), val) };
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(target_arch = "x86_64")]
impl<M: RamMapper> RamBuilder<'_, M> {
    pub fn build(
        mut self,
        _: &CpuFeats,
        page_size: NonZero<usize>,
        devices: &DeviceTree,
        dynamic: ProgramHeader,
    ) -> Result<RamMap, RamBuilderError> {
        // Allocate page-map level-4 table. We use 4K 4-Level Paging here. You may wonder about this
        // because it seems like page size on the PS4 is 16K. The truth is the PS4 emulate the 16K
        // page size with 4K pages. You can check this by yourself by looking at
        // acpi_install_wakeup_handler() function on the PS4 kernel and compare it with FreeBSD
        // version. No idea why the PS4 choose to emulate 16K page.
        //
        // See Page Translation and Protection section on AMD64 Architecture Programmer's Manual
        // Volume 2 for how paging work in long-mode.
        let (pml4t, page_table) = self
            .alloc_page_table()
            .map_err(RamBuilderError::AllocPml4TableFailed)?;
        let pml4t = unsafe { &mut *pml4t };

        // Setup page tables to map virtual devices. We use identity mapping for virtual devices.
        let mut dev_end = 0;

        for (addr, dev) in devices.all() {
            let len = dev.len().get();
            self.setup_4k_page_tables(pml4t, addr, addr, len)?;
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

        self.setup_4k_page_tables(pml4t, vaddr, kern_paddr, kern_len)?;

        vaddr += kern_len;

        // Setup page tables to map stack.
        let stack_vaddr = vaddr;
        let (paddr, stack_len) = self
            .stack
            .take()
            .map(|v| (v.start, v.end - v.start))
            .unwrap();

        self.setup_4k_page_tables(pml4t, vaddr, paddr, stack_len)?;

        vaddr += stack_len;

        // Setup page tables to map arguments.
        let args = self.args.take().unwrap();
        let ram = args.ram;
        let env_vaddr = vaddr + args.env;
        let conf_vaddr = vaddr + args.conf;

        self.setup_4k_page_tables(pml4t, vaddr, ram.start, ram.end - ram.start)?;

        // Relocate the kernel to virtual address.
        let map = RamMap {
            page_size,
            page_table,
            kern_paddr,
            kern_vaddr,
            kern_len,
            stack_vaddr,
            stack_len,
            env_vaddr,
            conf_vaddr,
        };

        unsafe { self.relocate_kernel(&map, dynamic, 8)? };

        Ok(map)
    }

    fn setup_4k_page_tables(
        &mut self,
        pml4t: &mut [usize; 512],
        vaddr: usize,
        paddr: usize,
        len: usize,
    ) -> Result<(), RamBuilderError> {
        let ram = self.ram.host_addr().cast_mut(); // TODO: Make this safer.

        assert_eq!(len % 4096, 0);

        fn set_page_entry(entry: &mut usize, addr: usize) {
            assert_eq!(addr & 0x7FF0000000000000, 0);
            assert_eq!(addr & 0xFFF, 0);

            *entry = addr;
            *entry |= 0b01; // Present (P) Bit.
            *entry |= 0b10; // Read/Write (R/W) Bit.
        }

        for off in (0..len).step_by(4096) {
            // Get page-directory pointer table.
            let addr = vaddr + off;
            let pml4o = (addr & 0xFF8000000000) >> 39;
            let pdpt = match pml4t[pml4o] {
                0 => {
                    let (pdpt, addr) = self
                        .alloc_page_table()
                        .map_err(RamBuilderError::AllocPdpTableFailed)?;

                    set_page_entry(&mut pml4t[pml4o], addr);

                    unsafe { &mut *pdpt }
                }
                v => unsafe { &mut *ram.add(v & 0xFFFFFFFFFF000).cast() },
            };

            // Get page-directory table.
            let pdpo = (addr & 0x7FC0000000) >> 30;
            let pdt = match pdpt[pdpo] {
                0 => {
                    let (pdt, addr) = self
                        .alloc_page_table()
                        .map_err(RamBuilderError::AllocPdTableFailed)?;

                    set_page_entry(&mut pdpt[pdpo], addr);

                    unsafe { &mut *pdt }
                }
                v => unsafe { &mut *ram.add(v & 0xFFFFFFFFFF000).cast() },
            };

            // Get page table.
            let pdo = (addr & 0x3FE00000) >> 21;
            let pt = match pdt[pdo] {
                0 => {
                    let (pt, addr) = self
                        .alloc_page_table()
                        .map_err(RamBuilderError::AllocPageTableFailed)?;

                    set_page_entry(&mut pdt[pdo], addr);

                    unsafe { &mut *pt }
                }
                v => unsafe { &mut *ram.add(v & 0xFFFFFFFFFF000).cast() },
            };

            // Set page table entry.
            let pto = (addr & 0x1FF000) >> 12;
            let addr = paddr + off;

            assert_eq!(pt[pto], 0);

            set_page_entry(&mut pt[pto], addr);
        }

        Ok(())
    }

    fn alloc_page_table(&mut self) -> Result<(*mut [usize; 512], usize), RamError> {
        // Get address and length.
        let addr = self.next;
        let len = (512usize * 8)
            .checked_next_multiple_of(self.ram.block_size().get())
            .and_then(NonZero::new)
            .unwrap();

        // Page table on x86-64 always 4k aligned regardless page size being used.
        assert_eq!(addr % 4096, 0);

        // Allocate.
        let tab = self
            .ram
            .alloc(addr, len)
            .map(|mut v| v.as_mut_ptr().cast())?;

        self.next += len.get();

        Ok((tab, addr))
    }
}

#[cfg(target_arch = "aarch64")]
impl<'a, M: RamMapper> RamBuilder<'a, M> {
    const MA_DEV_NG_NR_NE: u8 = 0; // MEMORY_ATTRS[0]
    const MA_NOR: u8 = 1; // MEMORY_ATTRS[1]
    const MEMORY_ATTRS: [u8; 8] = [0, 0b11111111, 0, 0, 0, 0, 0, 0];

    pub fn build(
        mut self,
        feats: &CpuFeats,
        page_size: NonZero<usize>,
        devices: &DeviceTree,
        dynamic: ProgramHeader,
    ) -> Result<RamMap, RamBuilderError> {
        // Setup page tables.
        let map = match page_size.get() {
            0x4000 => self.build_16k_page_tables(feats, devices)?,
            _ => todo!(),
        };

        // Relocate the kernel to virtual address.
        unsafe { self.relocate_kernel(&map, dynamic, 1027)? };

        // Flush modified memory.
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);

        Ok(map)
    }

    fn build_16k_page_tables(
        &mut self,
        feats: &CpuFeats,
        devices: &DeviceTree,
    ) -> Result<RamMap, RamBuilderError> {
        // Allocate page table level 0.
        let page_table = self.next;
        let len = self.ram.block_size();
        let l0t: &mut [usize; 32] = match self.ram.alloc(page_table, len) {
            Ok(mut v) => unsafe { &mut *v.as_mut_ptr().cast() },
            Err(e) => return Err(RamBuilderError::AllocPageTableLevel0Failed(e)),
        };

        self.next += len.get();

        // Map virtual devices. We use identity mapping for virtual devices.
        let mut dev_end = 0;

        for (addr, dev) in devices.all() {
            let len = dev.len().get();
            self.setup_16k_page_tables(feats, l0t, addr, addr, len, Self::MA_DEV_NG_NR_NE)?;
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

        self.setup_16k_page_tables(feats, l0t, vaddr, kern_paddr, kern_len, Self::MA_NOR)?;

        vaddr += kern_len;

        // Setup page tables to map stack.
        let stack_vaddr = vaddr;
        let (paddr, stack_len) = self
            .stack
            .take()
            .map(|v| (v.start, v.end - v.start))
            .unwrap();

        self.setup_16k_page_tables(feats, l0t, vaddr, paddr, stack_len, Self::MA_NOR)?;

        vaddr += stack_len;

        // Setup page tables to map arguments.
        let args = self.args.take().unwrap();
        let ram = args.ram;
        let env_vaddr = vaddr + args.env;
        let conf_vaddr = vaddr + args.conf;

        self.setup_16k_page_tables(
            feats,
            l0t,
            vaddr,
            ram.start,
            ram.end - ram.start,
            Self::MA_NOR,
        )?;

        Ok(RamMap {
            page_size: unsafe { NonZero::new_unchecked(0x4000) },
            page_table,
            memory_attrs: u64::from_le_bytes(Self::MEMORY_ATTRS),
            kern_paddr,
            kern_vaddr,
            kern_len,
            stack_vaddr,
            stack_len,
            env_vaddr,
            conf_vaddr,
        })
    }

    fn setup_16k_page_tables(
        &mut self,
        _: &CpuFeats,
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
                        .map_err(RamBuilderError::AllocPageTableLevel1Failed)?;

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
                        .map_err(RamBuilderError::AllocPageTableLevel2Failed)?;

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
                        .map_err(RamBuilderError::AllocPageTableLevel3Failed)?;

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

/// Contains information how kernel arguments was allocated.
struct KernelArgs {
    ram: Range<usize>,
    env: usize,
    conf: usize,
}

/// Struct to write all kernel arguments into a single block of memory.
struct ArgsWriter<'a, M: RamMapper> {
    mem: LockedMem<'a, M>,
    next: usize,
}

impl<M: RamMapper> ArgsWriter<'_, M> {
    fn write<T>(&mut self, v: T) -> usize {
        let off = self.next.next_multiple_of(align_of::<T>());

        self.mem.write(off, v);
        self.next = off + size_of::<T>();

        off
    }
}

/// Finalized layout of [`Ram`] before execute the kernel entry point.
pub struct RamMap {
    pub page_size: NonZero<usize>,
    pub page_table: usize,
    #[cfg(target_arch = "aarch64")]
    pub memory_attrs: u64,
    pub kern_paddr: usize,
    pub kern_vaddr: usize,
    pub kern_len: usize,
    pub stack_vaddr: usize,
    pub stack_len: usize,
    pub env_vaddr: usize,
    pub conf_vaddr: usize,
}

/// Represents an error when [`RamBuilder::build()`] fails
#[derive(Debug, Error)]
pub enum RamBuilderError {
    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate page-map level-4 table")]
    AllocPml4TableFailed(#[source] RamError),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate page-directory pointer table")]
    AllocPdpTableFailed(#[source] RamError),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate page-directory table")]
    AllocPdTableFailed(#[source] RamError),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't allocate page table")]
    AllocPageTableFailed(#[source] RamError),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't allocate page table level 0")]
    AllocPageTableLevel0Failed(#[source] RamError),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't allocate page table level 1")]
    AllocPageTableLevel1Failed(#[source] RamError),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't allocate page table level 2")]
    AllocPageTableLevel2Failed(#[source] RamError),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't allocate page table level 3")]
    AllocPageTableLevel3Failed(#[source] RamError),

    #[error("the kernel has invalid PT_DYNAMIC")]
    InvalidDynamicLinking,
}
