pub use self::page::*;
pub use self::stack::*;

use self::iter::StartFromMut;
use self::storage::Storage;
use crate::dev::DmemContainer;
use crate::errno::{Errno, EINVAL, ENOMEM, EOPNOTSUPP};
use crate::process::VThread;
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::{info, warn};
use bitflags::bitflags;
use macros::Errno;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fmt::{Display, Formatter};
use std::num::TryFromIntError;
use std::ptr::null_mut;
use std::sync::{Arc, RwLock};
use thiserror::Error;

mod iter;
mod page;
mod stack;
mod storage;

/// Implementation of `vmspace` structure.
#[derive(Debug)]
pub struct Vm {
    allocation_granularity: usize,
    allocations: RwLock<BTreeMap<usize, Alloc>>, // Key is Alloc::addr.
    stack: AppStack,
}

impl Vm {
    /// Size of a memory page on PS4.
    pub const VIRTUAL_PAGE_SIZE: usize = 0x4000;

    /// See `vmspace_alloc` on the PS4 for a reference.
    pub fn new(sys: &mut Syscalls) -> Result<Arc<Self>, MemoryManagerError> {
        // Check if page size on the host is supported. We don't need to check allocation
        // granularity because it is always multiply by page size, which is a correct value.
        let (page_size, allocation_granularity) = Self::get_memory_model();

        if page_size > Self::VIRTUAL_PAGE_SIZE {
            // If page size is larger than PS4 we will have a problem with memory protection.
            // Let's say page size on the host is 32K and we have 2 adjacent virtual pages, which is
            // 16K per virtual page. The first virtual page want to use read/write while the second
            // virtual page want to use read-only. This scenario will not be possible because those
            // two virtual pages are on the same page.
            return Err(MemoryManagerError::UnsupportedPageSize);
        }

        let mut mm = Self {
            allocation_granularity,
            allocations: RwLock::default(),
            stack: AppStack::new(),
        };

        // Allocate main stack.
        let guard = match mm.mmap(
            0,
            mm.stack.len() + Self::VIRTUAL_PAGE_SIZE,
            mm.stack.prot(),
            "main stack",
            MappingFlags::MAP_ANON | MappingFlags::MAP_PRIVATE,
            -1,
            0,
        ) {
            Ok(v) => v.into_raw(),
            Err(e) => return Err(MemoryManagerError::StackAllocationFailed(e)),
        };

        // Set the guard page to be non-accessible.
        if let Err(e) = mm.mprotect(guard, Self::VIRTUAL_PAGE_SIZE, Protections::empty()) {
            return Err(MemoryManagerError::GuardStackFailed(e));
        }

        mm.stack.set_guard(guard);
        mm.stack
            .set_stack(unsafe { guard.add(Self::VIRTUAL_PAGE_SIZE) });

        // Register syscall handlers.
        let mm = Arc::new(mm);

        sys.register(69, &mm, Self::sys_sbrk);
        sys.register(70, &mm, Self::sys_sstk);
        sys.register(73, &mm, Self::sys_munmap);
        sys.register(477, &mm, Self::sys_mmap);
        sys.register(548, &mm, Self::sys_batch_map);
        sys.register(588, &mm, Self::sys_mname);
        sys.register(628, &mm, Self::sys_mmap_dmem);

        Ok(mm)
    }

    pub fn stack(&self) -> &AppStack {
        &self.stack
    }

    pub fn mmap<N: Into<String>>(
        &self,
        addr: usize,
        len: usize,
        prot: Protections,
        name: N,
        mut flags: MappingFlags,
        fd: i32,
        offset: usize,
    ) -> Result<VPages<'_>, MmapError> {
        // Remove unknown protections.
        let prot = prot.intersection(Protections::all());

        // TODO: Check why the PS4 check RBP register.
        if flags.contains(MappingFlags::UNK1) {
            todo!("mmap with flags & 0x200000");
        }

        if len == 0 {
            todo!("mmap with len = 0");
        }

        if flags.intersects(MappingFlags::MAP_VOID | MappingFlags::MAP_ANON) {
            if offset != 0 {
                return Err(MmapError::NonZeroOffset);
            } else if fd != -1 {
                return Err(MmapError::NonNegativeFd);
            }
        } else if flags.contains(MappingFlags::MAP_STACK) {
            todo!("mmap with flags & 0x400");
        }

        flags.remove(MappingFlags::UNK2);
        flags.remove(MappingFlags::UNK3);

        // TODO: Refactor this for readability.
        let td = VThread::current();

        if ((offset & 0x3fff) ^ 0xffffffffffffbfff) < len {
            return Err(MmapError::InvalidOffset);
        }

        if flags.contains(MappingFlags::MAP_FIXED) {
            todo!("mmap with flags & 0x10");
        } else if addr == 0 {
            if td
                .as_ref()
                .is_some_and(|t| (t.proc().app_info().unk1() & 2) != 0)
            {
                todo!("mmap with addr = 0 and appinfo.unk1 & 2 != 0");
            }
        } else if (addr & 0xfffffffdffffffff) == 0 {
            // TODO: Check what the is value at offset 0x140 on vm_map.
        } else if addr == 0x880000000 {
            todo!("mmap with addr = 0x880000000");
        }

        if flags.contains(MappingFlags::MAP_VOID) {
            flags |= MappingFlags::MAP_ANON;

            if let Some(ref td) = td {
                td.set_fpop(None);
            }
        } else if !flags.contains(MappingFlags::MAP_ANON) {
            todo!("mmap with flags & 0x1000 = 0");
        }

        if flags.contains(MappingFlags::UNK1) {
            todo!("mmap with flags & 0x200000 != 0");
        }

        if td.is_some_and(|t| (t.proc().app_info().unk1() & 2) != 0) {
            todo!("mmap with addr = 0 and appinfo.unk1 & 2 != 0");
        }

        // Round len up to virtual page boundary.
        let len = match len % Self::VIRTUAL_PAGE_SIZE {
            0 => len,
            r => len + (Self::VIRTUAL_PAGE_SIZE - r),
        };

        self.map(addr, len, prot, name.into())
    }

    pub fn munmap(&self, addr: *mut u8, len: usize) -> Result<(), MunmapError> {
        // Check arguments.
        let first = addr as usize;

        if first % Self::VIRTUAL_PAGE_SIZE != 0 {
            return Err(MunmapError::UnalignedAddr);
        } else if len == 0 {
            return Err(MunmapError::ZeroLen);
        }

        // Do unmapping every pages in the range.
        let end = Self::align_virtual_page(unsafe { addr.add(len) });
        let mut adds: Vec<Alloc> = Vec::new();
        let mut removes: Vec<usize> = Vec::new();
        let mut allocs = self.allocations.write().unwrap();

        // FIXME: In theory it is possible to make this more efficient by remove allocation
        // info in-place. Unfortunately Rust does not provides API to achieve what we want.
        for (_, info) in StartFromMut::new(&mut allocs, first) {
            // Check if the current allocation is not in the range.
            if end <= info.addr {
                break;
            }

            // Check if we need to split the first allocation.
            if addr > info.addr {
                let remain = (info.end() as usize) - (addr as usize);

                // Check if we need to split in the middle.
                let decommit = if end < info.end() {
                    adds.push(Alloc {
                        addr: end,
                        len: (info.end() as usize) - (end as usize),
                        prot: info.prot,
                        name: info.name.clone(),
                        storage: info.storage.clone(),
                    });

                    (end as usize) - (addr as usize)
                } else {
                    remain
                };

                // Decommit the memory.
                if let Err(e) = info.storage.decommit(addr, decommit) {
                    panic!("Failed to decommit memory {addr:p}:{decommit}: {e}.");
                }

                info.len -= remain;
            } else if end < info.end() {
                // The current allocation is the last one in the region. What we do here is decommit
                // the head and keep the tail.
                let decommit = (end as usize) - (info.addr as usize);

                if let Err(e) = info.storage.decommit(info.addr, decommit) {
                    panic!(
                        "Failed to decommit memory {:p}:{}: {}.",
                        info.addr, decommit, e
                    );
                }

                // Split the region.
                removes.push(info.addr as usize);

                adds.push(Alloc {
                    addr: end,
                    len: info.len - decommit,
                    prot: info.prot,
                    name: info.name.clone(),
                    storage: info.storage.clone(),
                });
            } else {
                // Unmap the whole allocation.
                if let Err(e) = info.storage.decommit(info.addr, info.len) {
                    panic!(
                        "Failed to decommit memory {:p}:{}: {}.",
                        info.addr, info.len, e
                    );
                }

                removes.push(info.addr as usize);
            }
        }

        // Update allocation set.
        for alloc in adds {
            let addr = alloc.addr;

            if allocs.insert(addr as usize, alloc).is_some() {
                panic!("Address {addr:p} is already allocated.");
            }
        }

        for addr in removes {
            allocs.remove(&addr);
        }

        Ok(())
    }

    pub fn mprotect(
        &self,
        addr: *mut u8,
        len: usize,
        prot: Protections,
    ) -> Result<(), MemoryUpdateError> {
        self.update(
            addr,
            len,
            |i| i.prot != prot,
            |i| {
                i.storage.protect(i.addr, i.len, prot).unwrap();
                i.prot = prot;
            },
        )
    }

    /// See `vm_map_set_name` on the PS4 for a reference.
    pub fn mname(
        &self,
        addr: *mut u8,
        len: usize,
        name: impl AsRef<str>,
    ) -> Result<(), MemoryUpdateError> {
        let name = name.as_ref();
        let sname = CString::new(name);

        self.update(
            addr,
            len,
            |i| i.name != name,
            |i| {
                if let Ok(name) = &sname {
                    let _ = i.storage.set_name(i.addr, i.len, name);
                }

                i.name = name.to_owned();
            },
        )
    }

    /// See `vm_mmap` on the PS4 for a reference.
    fn map(
        &self,
        addr: usize,
        len: usize,
        prot: Protections,
        name: String,
    ) -> Result<VPages<'_>, MmapError> {
        // TODO: Check what is PS4 doing here.
        use std::collections::btree_map::Entry;

        // Do allocation.
        let addr = (addr + 0x3fff) & 0xffffffffffffc000;
        let alloc = match self.alloc(addr, len, prot, name) {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::OutOfMemory {
                    return Err(MmapError::NoMem(len));
                } else {
                    // We should not hit other error except for out of memory.
                    panic!("Failed to allocate {len} bytes: {e}.");
                }
            }
        };

        // Store allocation info.
        let mut allocs = self.allocations.write().unwrap();
        let alloc = match allocs.entry(alloc.addr as usize) {
            Entry::Occupied(e) => panic!("Address {:p} is already allocated.", e.key()),
            Entry::Vacant(e) => e.insert(alloc),
        };

        Ok(VPages::new(self, alloc.addr, alloc.len))
    }

    fn update<F, U>(
        &self,
        addr: *mut u8,
        len: usize,
        mut filter: F,
        mut update: U,
    ) -> Result<(), MemoryUpdateError>
    where
        F: FnMut(&Alloc) -> bool,
        U: FnMut(&mut Alloc),
    {
        // Check arguments.
        let first = addr as usize;

        if first % Self::VIRTUAL_PAGE_SIZE != 0 {
            return Err(MemoryUpdateError::UnalignedAddr);
        } else if len == 0 {
            return Err(MemoryUpdateError::ZeroLen);
        }

        // Get allocations within the range.
        let mut valid_addr = false;
        let end = Self::align_virtual_page(unsafe { addr.add(len) });
        let mut prev: *mut u8 = null_mut();
        let mut targets: Vec<&mut Alloc> = Vec::new();
        let mut allocs = self.allocations.write().unwrap();

        for (_, info) in StartFromMut::new(&mut allocs, first) {
            valid_addr = true;

            // Stop if the allocation is out of range.
            if end <= info.addr {
                break;
            }

            // TODO: Check if PS4 requires contiguous allocations.
            if !prev.is_null() && info.addr != prev {
                return Err(MemoryUpdateError::UnmappedAddr(prev));
            }

            prev = info.end();

            if filter(info) {
                targets.push(info);
            }
        }

        if !valid_addr {
            return Err(MemoryUpdateError::InvalidAddr);
        }

        // Update allocations within the range.
        let mut adds: Vec<Alloc> = Vec::new();

        for info in targets {
            let storage = &info.storage;

            // Check if we need to split the first allocation.
            if addr > info.addr {
                // Get how many bytes to split.
                let remain = (info.end() as usize) - (addr as usize);
                let len = if end < info.end() {
                    (end as usize) - (addr as usize)
                } else {
                    remain
                };

                // Split the first allocation.
                let mut alloc = Alloc {
                    addr,
                    len,
                    prot: info.prot,
                    name: info.name.clone(),
                    storage: storage.clone(),
                };

                update(&mut alloc);
                adds.push(alloc);

                // Check if the splitting was in the middle.
                if len != remain {
                    adds.push(Alloc {
                        addr: end,
                        len: (info.end() as usize) - (end as usize),
                        prot: info.prot,
                        name: info.name.clone(),
                        storage: storage.clone(),
                    });
                }

                info.len -= remain;
            } else if end < info.end() {
                // The current allocation is the last one in the range. What we do here is we split
                // the allocation and update the head.
                let tail = (info.end() as usize) - (end as usize);

                info.len -= tail;
                adds.push(Alloc {
                    addr: end,
                    len: tail,
                    prot: info.prot,
                    name: info.name.clone(),
                    storage: storage.clone(),
                });

                update(info);
            } else {
                // Update the whole allocation.
                update(info);
            }
        }

        // Add new allocation to the set.
        for alloc in adds {
            let addr = alloc.addr;
            assert!(allocs.insert(addr as usize, alloc).is_none());
        }

        Ok(())
    }

    fn alloc(
        &self,
        addr: usize,
        len: usize,
        prot: Protections,
        name: String,
    ) -> Result<Alloc, std::io::Error> {
        use self::storage::Memory;

        // Determine how to allocate.
        let (addr, len, storage) = if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that mean the result of
            // mmap may not aligned correctly. In this case we need to do 2 allocations. The first
            // allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/31411825/1829232
            let len = len + (Self::VIRTUAL_PAGE_SIZE - self.allocation_granularity);
            let storage = Memory::new(addr, len)?;

            // Do the second allocation.
            let addr = Self::align_virtual_page(storage.addr());
            let len = len - ((addr as usize) - (storage.addr() as usize));

            (addr, len, storage)
        } else {
            // If allocation granularity is equal or larger than the virtual page that mean the
            // result of mmap will always aligned correctly.
            let storage = Memory::new(addr, len)?;
            let addr = storage.addr();

            (addr, len, storage)
        };

        storage.commit(addr, len, prot)?;

        // Set storage name if supported.
        if let Ok(name) = CString::new(name.as_str()) {
            let _ = storage.set_name(addr, len, &name);
        }

        Ok(Alloc {
            addr,
            len,
            prot,
            name,
            storage: Arc::new(storage),
        })
    }

    fn sys_sbrk(self: &Arc<Self>, _: &VThread, _: &SysIn) -> Result<SysOut, SysErr> {
        // Return EOPNOTSUPP (Not yet implemented syscall)
        Err(SysErr::Raw(EOPNOTSUPP))
    }

    fn sys_sstk(self: &Arc<Self>, _: &VThread, _: &SysIn) -> Result<SysOut, SysErr> {
        // Return EOPNOTSUPP (Not yet implemented syscall)
        Err(SysErr::Raw(EOPNOTSUPP))
    }

    #[allow(unused_variables)]
    fn sys_munmap(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();

        self.munmap_internal(addr, len)
    }

    #[allow(unused_variables)]
    fn munmap_internal(self: &Arc<Self>, addr: usize, len: usize) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_mmap(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let prot: Protections = i.args[2].try_into().unwrap();
        let flags: MappingFlags = i.args[3].try_into().unwrap();
        let fd: i32 = i.args[4].try_into().unwrap();
        let pos: usize = i.args[5].into();

        self.mmap_internal(addr, len, prot, flags, fd, pos)
    }

    fn mmap_internal(
        self: &Arc<Self>,
        addr: usize,
        len: usize,
        prot: Protections,
        flags: MappingFlags,
        fd: i32,
        pos: usize,
    ) -> Result<SysOut, SysErr> {
        // Check if the request is a guard for main stack.
        if addr == self.stack.guard() {
            assert_eq!(len, Self::VIRTUAL_PAGE_SIZE);
            assert!(prot.is_empty());
            assert!(flags.intersects(MappingFlags::MAP_ANON));
            assert_eq!(fd, -1);
            assert_eq!(pos, 0);

            info!("Guard page has been requested for main stack.");

            return Ok(self.stack.guard().into());
        }

        // TODO: Make a proper name.
        let pages = self.mmap(addr, len, prot, "", flags, fd, pos)?;

        if addr != 0 && pages.addr() != addr {
            warn!(
                "mmap({:#x}, {:#x}, {}, {}, {}, {}) was success with {:#x} instead of {:#x}.",
                addr,
                len,
                prot,
                flags,
                fd,
                pos,
                pages.addr(),
                addr
            );
        } else {
            info!(
                "{:#x}:{:p} is mapped as {} with {}.",
                pages.addr(),
                pages.end(),
                prot,
                flags,
            );
        }

        Ok(pages.into_raw().into())
    }

    fn sys_batch_map(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let dmem_fd: i32 = i.args[0].try_into().unwrap();
        let flags: MappingFlags = i.args[1].try_into().unwrap();
        let operations: *const BatchMapArg = i.args[2].into();
        let num_of_ops: i32 = i.args[3].try_into().unwrap();
        let num_of_processed_ops: *mut i32 = i.args[4].into();

        if flags.bits() & 0xe0bffb6f != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        let slice_size = num_of_ops.try_into().ok().ok_or(SysErr::Raw(EINVAL))?;
        let operations = unsafe { std::slice::from_raw_parts(operations, slice_size) };

        let mut processed = 0;

        let result = operations.iter().try_for_each(|arg| {
            match arg.op.try_into()? {
                BatchMapOp::MapDirect => {
                    if *td.proc().dmem_container() != DmemContainer::One
                    /* || td.proc().unk4 & 2 != 0 */
                    /* || td.proc().sdk_version < 0x2500000 */
                    || flags.intersects(MappingFlags::MAP_STACK)
                    {
                        todo!()
                    }

                    self.mmap_dmem_internal(
                        arg.addr,
                        arg.len,
                        arg.ty.try_into().unwrap(),
                        arg.prot.try_into().unwrap(),
                        flags,
                        arg.offset,
                    )?;
                }
                BatchMapOp::MapFlexible => {
                    if arg.addr & 0x3fff != 0 || arg.len & 0x3fff != 0 || arg.prot & 0xc8 != 0 {
                        return Err(SysErr::Raw(EINVAL));
                    }

                    self.mmap_internal(
                        arg.addr,
                        arg.len,
                        arg.prot.try_into().unwrap(),
                        flags.intersection(MappingFlags::MAP_ANON),
                        -1,
                        0,
                    )?;
                }
                BatchMapOp::Protect => todo!(),
                BatchMapOp::TypeProtect => todo!(),
                BatchMapOp::Unmap => {
                    if arg.addr & 0x3fff != 0 || arg.len & 0x3fff != 0 {
                        return Err(SysErr::Raw(EINVAL));
                    }

                    self.munmap_internal(arg.addr, arg.len)?;
                }
                _ => todo!(),
            }

            processed = processed + 1;

            Ok(())
        });

        // TODO: invalidate TLB

        if !num_of_processed_ops.is_null() {
            unsafe {
                *num_of_processed_ops = processed;
            }
        }

        result.map(|_| SysOut::ZERO)
    }

    fn sys_mname(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let name = unsafe { i.args[2].to_str(32)?.unwrap() };

        info!(
            "Setting name for {:#x}:{:#x} to '{}'.",
            addr,
            addr + len,
            name
        );

        // PS4 does not check if vm_map_set_name is failed.
        let len = ((addr & 0x3fff) + len + 0x3fff) & 0xffffffffffffc000;
        let addr = (addr & 0xffffffffffffc000) as *mut u8;

        if let Err(e) = self.mname(addr, len, name) {
            warn!(e, "mname({addr:p}, {len:#x}, {name}) failed");
        }

        Ok(SysOut::ZERO)
    }

    #[allow(unused_variables)]
    fn sys_mmap_dmem(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let start_addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let mem_type: MemoryType = i.args[2].try_into().unwrap();
        let prot: Protections = i.args[3].try_into().unwrap();
        let flags: MappingFlags = i.args[4].try_into().unwrap();
        let start_phys_addr: usize = i.args[5].into();

        self.mmap_dmem_internal(start_addr, len, mem_type, prot, flags, start_phys_addr)
    }

    #[allow(unused_variables)]
    fn mmap_dmem_internal(
        self: &Arc<Self>,
        start_addr: usize,
        len: usize,
        mem_type: MemoryType,
        prot: Protections,
        flags: MappingFlags,
        start_phys_addr: usize,
    ) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn align_virtual_page(ptr: *mut u8) -> *mut u8 {
        match (ptr as usize) % Self::VIRTUAL_PAGE_SIZE {
            0 => ptr,
            v => unsafe { ptr.add(Self::VIRTUAL_PAGE_SIZE - v) },
        }
    }

    #[cfg(unix)]
    fn get_memory_model() -> (usize, usize) {
        let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

        if v < 0 {
            let e = std::io::Error::last_os_error();
            panic!("Failed to get page size: {e}.");
        }

        (v as usize, v as usize)
    }

    #[cfg(windows)]
    fn get_memory_model() -> (usize, usize) {
        use std::mem::MaybeUninit;
        use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
        let mut i = MaybeUninit::<SYSTEM_INFO>::uninit();

        unsafe { GetSystemInfo(i.as_mut_ptr()) };

        let i = unsafe { i.assume_init() };

        (i.dwPageSize as usize, i.dwAllocationGranularity as usize)
    }
}

unsafe impl Sync for Vm {}

/// Contains information for an allocation of virtual pages.
#[derive(Debug)]
struct Alloc {
    addr: *mut u8,
    len: usize,
    prot: Protections,
    name: String,
    storage: Arc<dyn Storage>,
}

impl Alloc {
    fn end(&self) -> *mut u8 {
        unsafe { self.addr.add(self.len) }
    }
}

unsafe impl Send for Alloc {}

bitflags! {
    /// Flags to tell what access is possible for the virtual page.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Protections: u32 {
        const CPU_READ = 0x00000001;
        const CPU_WRITE = 0x00000002;
        const CPU_EXEC = 0x00000004;
        const CPU_MASK = Self::CPU_READ.bits() | Self::CPU_WRITE.bits() | Self::CPU_EXEC.bits();
        const GPU_READ = 0x00000010;
        const GPU_WRITE = 0x00000020;
        const GPU_MASK = Self::GPU_READ.bits() | Self::GPU_WRITE.bits();
    }
}

impl Protections {
    #[cfg(unix)]
    fn into_host(self) -> std::ffi::c_int {
        use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};

        let mut host = PROT_NONE;

        if self.contains(Self::CPU_READ) {
            host |= PROT_READ;
        }

        if self.contains(Self::CPU_WRITE) {
            host |= PROT_WRITE;
        }

        if self.contains(Self::CPU_EXEC) {
            host |= PROT_EXEC;
        }

        host
    }

    #[cfg(windows)]
    fn into_host(self) -> windows_sys::Win32::System::Memory::PAGE_PROTECTION_FLAGS {
        use windows_sys::Win32::System::Memory::{
            PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_NOACCESS, PAGE_READONLY,
            PAGE_READWRITE,
        };

        // We cannot use "match" here because we need "|" to do bitwise OR.
        let cpu = self & Self::CPU_MASK;

        if cpu == Self::CPU_EXEC {
            PAGE_EXECUTE
        } else if cpu == Self::CPU_EXEC | Self::CPU_READ {
            PAGE_EXECUTE_READ
        } else if cpu == Self::CPU_EXEC | Self::CPU_READ | Self::CPU_WRITE {
            PAGE_EXECUTE_READWRITE
        } else if cpu == Self::CPU_READ {
            PAGE_READONLY
        } else if cpu == (Self::CPU_READ | Self::CPU_WRITE) || cpu == Self::CPU_WRITE {
            PAGE_READWRITE
        } else {
            PAGE_NOACCESS
        }
    }
}

impl TryFrom<SysArg> for Protections {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(v.get().try_into()?))
    }
}

impl TryFrom<u8> for Protections {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(value as u32))
    }
}

impl Display for Protections {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

bitflags! {
    /// Flags for [`MemoryManager::mmap()`].
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct MappingFlags: u32 {
        const MAP_PRIVATE = 0x00000002;
        const MAP_FIXED = 0x00000010;
        const MAP_VOID = 0x00000100;
        const MAP_STACK = 0x00000400;
        const MAP_ANON = 0x00001000;
        const MAP_GUARD = 0x00002000;
        const UNK2 = 0x00010000;
        const UNK3 = 0x00100000;
        const UNK1 = 0x00200000;
    }
}

impl TryFrom<SysArg> for MappingFlags {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(v.get().try_into()?))
    }
}

impl Display for MappingFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents an error when [`MemoryManager`] is failed to initialize.
#[derive(Debug, Error)]
pub enum MemoryManagerError {
    #[error("host system is using an unsupported page size")]
    UnsupportedPageSize,

    #[error("cannot allocate main stack")]
    StackAllocationFailed(#[source] MmapError),

    #[error("cannot setup guard page for main stack")]
    GuardStackFailed(#[source] MemoryUpdateError),
}

/// Represents an error when [`MemoryManager::mmap()`] is failed.
#[derive(Debug, Error, Errno)]
pub enum MmapError {
    #[error("MAP_ANON is specified with non-negative file descriptor")]
    #[errno(EINVAL)]
    NonNegativeFd,

    #[error("MAP_ANON is specified with non-zero offset")]
    #[errno(EINVAL)]
    NonZeroOffset,

    #[error("invalid offset")]
    #[errno(EINVAL)]
    InvalidOffset,

    #[error("no memory available for {0} bytes")]
    #[errno(ENOMEM)]
    NoMem(usize),
}

/// Errors for [`MemoryManager::munmap()`].
#[derive(Debug, Error, Errno)]
pub enum MunmapError {
    #[error("addr is not aligned")]
    #[errno(EINVAL)]
    UnalignedAddr,

    #[error("len is zero")]
    #[errno(EINVAL)]
    ZeroLen,
}

/// Represents an error when update operations on the memory is failed.
#[derive(Debug, Error)]
pub enum MemoryUpdateError {
    #[error("addr is not aligned")]
    UnalignedAddr,

    #[error("len is zero")]
    ZeroLen,

    #[error("invalid addr")]
    InvalidAddr,

    #[error("address {0:p} is not mapped")]
    UnmappedAddr(*const u8),
}

#[repr(C)]
struct BatchMapArg {
    addr: usize,
    offset: usize,
    len: usize,
    prot: u8,
    ty: u8,
    op: i32,
}

#[repr(i32)]
enum BatchMapOp {
    MapDirect = 0,
    Unmap = 1,
    Protect = 2,
    MapFlexible = 3,
    TypeProtect = 4,
}

impl TryFrom<i32> for BatchMapOp {
    type Error = SysErr;

    fn try_from(raw: i32) -> Result<Self, SysErr> {
        match raw {
            0 => Ok(BatchMapOp::MapDirect),
            1 => Ok(BatchMapOp::Unmap),
            2 => Ok(BatchMapOp::Protect),
            3 => Ok(BatchMapOp::MapFlexible),
            4 => Ok(BatchMapOp::TypeProtect),
            _ => Err(SysErr::Raw(EINVAL)),
        }
    }
}

#[repr(i32)]
enum MemoryType {
    WbOnion = 0,
    WcGarlic = 3,
    WbGarlic = 10,
}

impl TryFrom<SysArg> for MemoryType {
    type Error = TryFromIntError;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        let val: u8 = value.try_into().unwrap();
        val.try_into()
    }
}

impl TryFrom<u8> for MemoryType {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MemoryType::WbOnion),
            3 => Ok(MemoryType::WcGarlic),
            10 => Ok(MemoryType::WbGarlic),
            _ => unreachable!(),
        }
    }
}
