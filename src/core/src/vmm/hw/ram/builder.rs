use super::{Ram, RamError};
use crate::vmm::VmmError;
use obconf::BootEnv;
use std::ops::Range;
use thiserror::Error;

/// Struct to build [`Ram`].
pub struct RamBuilder {
    ram: Ram,
    next: usize,
    kern: Option<Range<usize>>,
    stack: Option<Range<usize>>,
    args: Option<KernelArgs>,
}

impl RamBuilder {
    pub fn new() -> Result<Self, VmmError> {
        // In theory we can support any page size on the host. The problem is it required a lot of
        // work. It is also unlikely for someone to need this feature because AFAIK the maximum page
        // size on a consumer computer is the same as PS4. With page size the same as PS4 or lower
        // we don't need to keep track allocations here.
        let page_size = Self::get_page_size().map_err(VmmError::GetPageSizeFailed)?;

        if page_size > Ram::VM_PAGE_SIZE {
            return Err(VmmError::UnsupportedPageSize);
        }

        // Reserve memory range.
        #[cfg(unix)]
        let mem = {
            use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};
            use std::ptr::null_mut;

            let mem = unsafe {
                mmap(
                    null_mut(),
                    Ram::SIZE,
                    PROT_NONE,
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                )
            };

            if mem == MAP_FAILED {
                return Err(VmmError::CreateRamFailed(std::io::Error::last_os_error()));
            }

            mem.cast()
        };

        #[cfg(windows)]
        let mem = {
            use std::ptr::null;
            use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

            let mem = unsafe { VirtualAlloc(null(), Ram::SIZE, MEM_RESERVE, PAGE_NOACCESS) };

            if mem.is_null() {
                return Err(VmmError::CreateRamFailed(std::io::Error::last_os_error()));
            }

            mem.cast()
        };

        Ok(Self {
            ram: Ram(mem),
            next: 0,
            kern: None,
            stack: None,
            args: None,
        })
    }

    /// # Panics
    /// If called a second time.
    pub fn alloc_kernel(&mut self) -> KernelBuilder {
        let start = self.next;

        assert!(self.kern.is_none());

        KernelBuilder {
            rb: self,
            start,
            len: 0,
        }
    }

    /// # Panics
    /// - If `len` is not multiplied by [`Ram::VM_PAGE_SIZE`].
    /// - If called a second time.
    pub fn alloc_stack(&mut self, len: usize) -> Result<(), RamError> {
        assert!(self.stack.is_none());

        unsafe { self.ram.alloc(self.next, len) }?;

        self.stack = Some(self.next..(self.next + len));
        self.next += len;

        Ok(())
    }

    /// # Panics
    /// If called a second time.
    pub fn alloc_args(&mut self, env: BootEnv) -> Result<(), RamError> {
        assert!(self.args.is_none());
        assert!(align_of::<BootEnv>() <= Ram::VM_PAGE_SIZE);

        // Allocate RAM for all arguments.
        let len = size_of::<BootEnv>().next_multiple_of(Ram::VM_PAGE_SIZE);
        let args = unsafe { self.ram.alloc(self.next, len)?.as_mut_ptr() };

        // Write env.
        let off = 0;

        unsafe { std::ptr::write(args.cast(), env) };

        self.args = Some(KernelArgs {
            ram: self.next..(self.next + len),
            env: off,
        });

        self.next += len;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    pub fn build(
        mut self,
        dynamic: Option<(usize, usize)>,
    ) -> Result<(Ram, RamMap), RamBuilderError> {
        // For x86-64 we require the kernel to be a Position-Independent Executable so we can map it
        // at the same address as the PS4 kernel.
        let dynamic = dynamic.ok_or(RamBuilderError::NonPieKernel)?;

        // Allocate page-map level-4 table. We use 4K 4-Level Paging here. Not sure how the PS4
        // achieve 16K page because x86-64 does not support it. Maybe it is a special request from
        // Sony to AMD?
        //
        // See Page Translation and Protection section on AMD64 Architecture Programmer's Manual
        // Volume 2 for how paging work in long-mode.
        let (pml4t, page_table) = self
            .alloc_page_table()
            .map_err(RamBuilderError::AllocPml4TableFailed)?;
        let pml4t = unsafe { &mut *pml4t };

        // Setup page tables to map virtual address 0xffffffff82200000 to the kernel.
        // TODO: Implement ASLR.
        let mut vaddr = 0xffffffff82200000;
        let kern_vaddr = vaddr;
        let (kern_paddr, kern_len) = self
            .kern
            .take()
            .map(|v| (v.start, v.end - v.start))
            .unwrap();

        self.setup_page_tables(pml4t, vaddr, kern_paddr, kern_len)?;

        vaddr += kern_len;

        // Setup page tables to map stack.
        let stack_vaddr = vaddr;
        let (paddr, stack_len) = self
            .stack
            .take()
            .map(|v| (v.start, v.end - v.start))
            .unwrap();

        self.setup_page_tables(pml4t, vaddr, paddr, stack_len)?;

        vaddr += stack_len;

        // Setup page tables to map arguments.
        let args = self.args.take().unwrap();
        let ram = args.ram;
        let env_vaddr = vaddr + args.env;

        self.setup_page_tables(pml4t, vaddr, ram.start, ram.end - ram.start)?;

        // Check if PT_DYNAMIC valid.
        let (p_vaddr, p_memsz) = dynamic;

        if p_memsz % 16 != 0 {
            return Err(RamBuilderError::InvalidDynamicLinking);
        }

        // Get PT_DYNAMIC.
        let kern = unsafe { std::slice::from_raw_parts_mut(self.ram.0.add(kern_paddr), kern_len) };
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

        // Relocate the kernel to virtual address.
        match (rela, relasz) {
            (None, None) => {}
            (Some(rela), Some(relasz)) => Self::relocate_kernel(kern, kern_vaddr, rela, relasz)?,
            _ => return Err(RamBuilderError::InvalidDynamicLinking),
        }

        // Build map.
        let map = RamMap {
            page_table,
            kern_vaddr,
            kern_len,
            stack_vaddr,
            stack_len,
            env_vaddr,
        };

        Ok((self.ram, map))
    }

    #[cfg(target_arch = "aarch64")]
    pub fn build(self, dynamic: Option<(usize, usize)>) -> Result<(Ram, RamMap), RamBuilderError> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn relocate_kernel(
        kern: &mut [u8],
        vaddr: usize,
        relocs: usize,
        len: usize,
    ) -> Result<(), RamBuilderError> {
        // Check if size valid.
        if (len % 24) != 0 || !relocs.checked_add(len).is_some_and(|end| end <= kern.len()) {
            return Err(RamBuilderError::InvalidDynamicLinking);
        }

        // Apply relocations.
        for off in (0..len).step_by(24).map(|v| relocs + v) {
            let r_offset = usize::from_ne_bytes(kern[off..(off + 8)].try_into().unwrap());
            let r_info = usize::from_ne_bytes(kern[(off + 8)..(off + 16)].try_into().unwrap());
            let r_addend = isize::from_ne_bytes(kern[(off + 16)..(off + 24)].try_into().unwrap());

            match r_info & 0xffffffff {
                // R_X86_64_NONE
                0 => break,
                // R_X86_64_RELATIVE
                8 => {
                    let dst = r_offset
                        .checked_add(8)
                        .and_then(|end| kern.get_mut(r_offset..end))
                        .ok_or(RamBuilderError::InvalidDynamicLinking)?;
                    let val = vaddr.wrapping_add_signed(r_addend);

                    unsafe { core::ptr::write_unaligned(dst.as_mut_ptr().cast(), val) };
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn setup_page_tables(
        &mut self,
        pml4t: &mut [usize; 512],
        vaddr: usize,
        paddr: usize,
        len: usize,
    ) -> Result<(), RamBuilderError> {
        assert_eq!(len % Ram::VM_PAGE_SIZE, 0);

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
                v => unsafe { &mut *self.ram.0.add(v & 0xFFFFFFFFFF000).cast() },
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
                v => unsafe { &mut *self.ram.0.add(v & 0xFFFFFFFFFF000).cast() },
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
                v => unsafe { &mut *self.ram.0.add(v & 0xFFFFFFFFFF000).cast() },
            };

            // Set page table entry.
            let pto = (addr & 0x1FF000) >> 12;
            let addr = paddr + off;

            assert_eq!(pt[pto], 0);

            set_page_entry(&mut pt[pto], addr);
        }

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn alloc_page_table(&mut self) -> Result<(*mut [usize; 512], usize), RamError> {
        let off = self.next;
        let len = (512usize * 8).next_multiple_of(Ram::VM_PAGE_SIZE);
        let tab = unsafe { self.ram.alloc(off, len).map(|v| v.as_mut_ptr().cast())? };

        self.next += len;

        Ok((tab, off))
    }

    #[cfg(unix)]
    fn get_page_size() -> Result<usize, std::io::Error> {
        let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

        if v < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(v.try_into().unwrap())
        }
    }

    #[cfg(windows)]
    fn get_page_size() -> Result<usize, std::io::Error> {
        use std::mem::zeroed;
        use windows_sys::Win32::System::SystemInformation::GetSystemInfo;
        let mut i = unsafe { zeroed() };

        unsafe { GetSystemInfo(&mut i) };

        Ok(i.dwPageSize.try_into().unwrap())
    }
}

/// Struct to allocate RAM to map the kernel.
pub struct KernelBuilder<'a> {
    rb: &'a mut RamBuilder,
    start: usize,
    len: usize,
}

impl<'a> KernelBuilder<'a> {
    pub fn alloc_segment(&mut self, addr: usize, mut len: usize) -> Result<&mut [u8], RamError> {
        // Check if addr valid.
        if (addr % Ram::VM_PAGE_SIZE) != 0 {
            return Err(RamError::UnalignedAddr);
        } else if addr < self.len {
            return Err(RamError::OverlappedAddr);
        }

        len = len
            .checked_next_multiple_of(Ram::VM_PAGE_SIZE)
            .ok_or(RamError::InvalidLen)?;

        // Allocate.
        let off = self.start.checked_add(addr).ok_or(RamError::InvalidAddr)?;
        let seg = unsafe { self.rb.ram.alloc(off, len)? };

        self.len = addr + len;

        Ok(seg)
    }
}

impl<'a> Drop for KernelBuilder<'a> {
    fn drop(&mut self) {
        self.rb.kern = Some(self.start..(self.start + self.len));
        self.rb.next += self.len;
    }
}

/// Contains information how kernel arguments was allocated.
pub struct KernelArgs {
    ram: Range<usize>,
    env: usize,
}

/// Finalized layout of [`Ram`] before execute the kernel entry point.
pub struct RamMap {
    pub page_table: usize,
    pub kern_vaddr: usize,
    pub kern_len: usize,
    pub stack_vaddr: usize,
    pub stack_len: usize,
    pub env_vaddr: usize,
}

/// Represents an error when [`RamBuilder::build()`] fails
#[derive(Debug, Error)]
pub enum RamBuilderError {
    #[cfg(target_arch = "x86_64")]
    #[error("the kernel is not a position-independent executable")]
    NonPieKernel,

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

    #[cfg(target_arch = "x86_64")]
    #[error("the kernel has invalid PT_DYNAMIC")]
    InvalidDynamicLinking,
}
