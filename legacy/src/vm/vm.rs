use super::iter::StartFromMut;
use super::{Alloc, AppStack, MappingFlags, MemoryType, Protections, VPages};
use crate::errno::{Errno, EINVAL, ENOMEM};
use crate::process::VThread;
use crate::syscalls::{SysErr, SysOut};
use crate::{info, warn};
use macros::Errno;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::ptr::null_mut;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Implementation of `vmspace` structure.
#[derive(Debug)]
pub struct VmSpace {
    allocation_granularity: usize,
    allocations: RwLock<BTreeMap<usize, Alloc>>, // Key is Alloc::addr.
    stack: AppStack,
}

impl VmSpace {
    /// Size of a memory page on PS4.
    pub const VIRTUAL_PAGE_SIZE: usize = 0x4000;

    /// See `vmspace_alloc` on the PS4 for a reference.
    pub fn new() -> Result<Arc<Self>, MemoryManagerError> {
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

        let mm = Arc::new(mm);

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
                return Err(MemoryUpdateError::UnmappedAddr(prev as _));
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
        use super::storage::{Memory, Storage};

        // Determine how to allocate.
        let (addr, len, storage) = if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that means the result of
            // mmap may not be aligned correctly. In this case we need to do 2 allocations. The first
            // allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/31411825/1829232
            let len = len + (Self::VIRTUAL_PAGE_SIZE - self.allocation_granularity);
            let storage = Memory::new(addr, len)?;

            // Do the second allocation.
            let addr = Self::align_virtual_page(storage.addr());
            let len = len - ((addr as usize) - (storage.addr() as usize));

            (addr, len, storage)
        } else {
            // If allocation granularity is equal or larger than the virtual page, that means the
            // result of mmap will always be aligned correctly.
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

    #[allow(unused_variables)]
    pub fn munmap_internal(self: &Arc<Self>, addr: usize, len: usize) -> Result<SysOut, SysErr> {
        todo!()
    }

    pub fn mmap_internal(
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

    #[allow(unused_variables)]
    pub fn mmap_dmem_internal(
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

unsafe impl Sync for VmSpace {}

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

    #[error("address {0:#x} is not mapped")]
    UnmappedAddr(usize),
}
