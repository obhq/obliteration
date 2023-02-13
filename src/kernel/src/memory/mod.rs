use self::iter::StartFromMut;
use self::storage::Storage;
use crate::errno::{EINVAL, ENOMEM};
use crate::syserr;
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::ptr::null_mut;
use std::sync::{Arc, RwLock};
use thiserror::Error;

pub mod iter;
pub mod storage;

/// Manage all paged memory that can be seen by a PS4 app.
pub struct MemoryManager {
    page_size: usize,
    allocation_granularity: usize,
    allocations: RwLock<BTreeMap<usize, Alloc>>, // Key is Alloc::addr.
}

impl MemoryManager {
    /// Size of a memory page on PS4.
    pub const VIRTUAL_PAGE_SIZE: usize = 0x4000;

    pub(super) fn new() -> Self {
        // Check if page size on the host is supported. We don't need to check allocation
        // granularity because it is always multiply of page size, which is a correct value.
        let (page_size, allocation_granularity) = Self::get_memory_model();

        if page_size > Self::VIRTUAL_PAGE_SIZE || (Self::VIRTUAL_PAGE_SIZE % page_size) != 0 {
            // If page size is larger than PS4 we will have a problem with memory protection.
            // Let's say page size on the host is 32K and we have 2 adjacent virtual pages, which is
            // 16K per virtual page. The first virtual page want to use read/write while the second
            // virtual page want to use read-only. This scenario will not be possible because those
            // two virtual pages are on the same page.
            panic!("Your system is using an unsupported page size.");
        }

        Self {
            page_size,
            allocation_granularity,
            allocations: RwLock::new(BTreeMap::new()),
        }
    }

    /// Gets size of page on the host system.
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// Gets allocation granularity on the host system.
    pub fn allocation_granularity(&self) -> usize {
        self.allocation_granularity
    }

    pub fn mmap(
        &self,
        addr: usize,
        len: usize,
        prot: Protections,
        flags: MappingFlags,
        fd: i32,
        offset: i64,
    ) -> Result<*mut u8, MmapError> {
        // Chech addr and len.
        if addr != 0 {
            syserr!("non-zero addr is not supported yet");
        } else if len == 0 {
            return Err(MmapError::ZeroLen);
        }

        // Check prot.
        if prot.contains_unknown() {
            syserr!("unknown prot {:#010x}", prot);
        }

        // Check if either MAP_SHARED or MAP_PRIVATE is specified. not both.
        // See https://stackoverflow.com/a/39945292/1829232 for how each flag is working.
        let is_private = match flags.behavior() {
            MappingFlags::BEHAVIOR_NONE => return Err(MmapError::NoBehavior),
            MappingFlags::BEHAVIOR_SHARED => false,
            MappingFlags::BEHAVIOR_PRIVATE => true,
            _ => return Err(MmapError::InvalidBehavior),
        };

        // Check for other flags if we are supported.
        if flags.alignment() != 0 {
            syserr!("MAP_ALIGNED or MAP_ALIGNED_SUPER is not supported yet");
        } else if flags.contains(MappingFlags::MAP_FIXED) {
            syserr!("MAP_FIXED is not supported yet");
        } else if flags.contains(MappingFlags::MAP_HASSEMAPHORE) {
            syserr!("MAP_HASSEMAPHORE is not supported yet");
        } else if flags.contains(MappingFlags::MAP_NOCORE) {
            syserr!("MAP_NOCORE is not supported yet");
        } else if flags.contains(MappingFlags::MAP_NOSYNC) {
            syserr!("MAP_NOSYNC is not support yet");
        } else if flags.contains(MappingFlags::MAP_PREFAULT_READ) {
            syserr!("MAP_PREFAULT_READ is not supported yet");
        } else if !is_private {
            syserr!("MAP_SHARED is not supported yet");
        } else if flags.contains(MappingFlags::MAP_STACK) {
            syserr!("MAP_STACK is not supported yet");
        }

        // Round len up to virtual page boundary.
        let len = match len % Self::VIRTUAL_PAGE_SIZE {
            0 => len,
            r => len + (Self::VIRTUAL_PAGE_SIZE - r),
        };

        // Check mapping source.
        if flags.contains(MappingFlags::MAP_ANON) {
            self.mmap_anon(len, prot, fd, offset)
        } else {
            syserr!("non-anonymous mapping is not supported yet");
        }
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
                        storage: info.storage.clone(),
                    });

                    (end as usize) - (addr as usize)
                } else {
                    remain
                };

                // Decommit the memory.
                if let Err(e) = info.storage.decommit(addr, decommit) {
                    panic!(
                        "Failed to decommit the memory {:p}:{}: {}.",
                        addr, decommit, e
                    );
                }

                info.len -= remain;
            } else if end < info.end() {
                // The current allocation is the last one in the region. What we do here is decommit
                // the head and keep the tail.
                let decommit = (end as usize) - (info.addr as usize);

                if let Err(e) = info.storage.decommit(info.addr, decommit) {
                    panic!(
                        "Failed to decommit the memory {:p}:{}: {}.",
                        info.addr, decommit, e
                    );
                }

                // Split the region.
                removes.push(info.addr as usize);

                adds.push(Alloc {
                    addr: end,
                    len: info.len - decommit,
                    prot: info.prot,
                    storage: info.storage.clone(),
                });
            } else {
                // Unmap the whole allocation.
                if let Err(e) = info.storage.decommit(info.addr, info.len) {
                    panic!(
                        "Failed to decommit the memory {:p}:{}: {}.",
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
                panic!("Address {:p} is already allocated.", addr);
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
    ) -> Result<(), MprotectError> {
        // Check arguments. FreeBSD man page does not specify if addr should be page-aligned or len
        // should not be zero but the Linux man page has this requirements. So let's follow the
        // Linux man page until some games expect a different behavior.
        let first = addr as usize;

        if first % Self::VIRTUAL_PAGE_SIZE != 0 {
            return Err(MprotectError::UnalignedAddr);
        } else if len == 0 {
            return Err(MprotectError::ZeroLen);
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

            // Check if the current allocation is contiguous with the previous one. FreeBSD man page
            // did not specify this requirements but Linux is specified.
            if !prev.is_null() && info.addr != prev {
                return Err(MprotectError::UnmappedAddr(prev));
            }

            prev = info.end();

            // If the current protection is the same we don't need to do anything.
            if info.prot != prot {
                targets.push(info);
            }
        }

        if !valid_addr {
            return Err(MprotectError::InvalidAddr);
        }

        // Change protection for allocations in the range.
        let mut adds: Vec<Alloc> = Vec::new();

        for info in targets {
            let storage = &info.storage;

            // Check if we need to split the first allocation.
            if addr > info.addr {
                // Split the allocation.
                let remain = (info.end() as usize) - (addr as usize);
                let len = if end < info.end() {
                    (end as usize) - (addr as usize)
                } else {
                    remain
                };

                adds.push(Alloc {
                    addr,
                    len,
                    prot,
                    storage: storage.clone(),
                });

                // Change protection.
                if let Err(e) = storage.protect(addr, len, prot) {
                    panic!(
                        "Failed to change protection on {:p}:{} to {:?}: {}",
                        addr, len, prot, e
                    );
                }

                // Check if the splitting was in the middle.
                if len != remain {
                    adds.push(Alloc {
                        addr: end,
                        len: (info.end() as usize) - (end as usize),
                        prot: info.prot,
                        storage: storage.clone(),
                    });
                }

                info.len -= remain;
            } else if end < info.end() {
                // The current allocation is the last one in the range. What we do here is we split
                // the allocation and change the protection on the head only.
                let remain = (info.end() as usize) - (end as usize);
                let change = info.len - remain;

                if let Err(e) = storage.protect(info.addr, change, prot) {
                    panic!(
                        "Failed to change protection on {:p}:{} to {:?}: {}",
                        info.addr, change, prot, e
                    );
                }

                // Split the tail.
                adds.push(Alloc {
                    addr: end,
                    len: remain,
                    prot: info.prot,
                    storage: storage.clone(),
                });

                info.len = change;
                info.prot = prot;
            } else {
                // Change protection the whole allocation.
                if let Err(e) = storage.protect(info.addr, info.len, prot) {
                    panic!(
                        "Failed to change protection on {:p}:{} to {:?}: {}",
                        info.addr, info.len, prot, e
                    );
                }

                info.prot = prot;
            }
        }

        // Add new allocation to the set.
        for alloc in adds {
            let addr = alloc.addr;

            if allocs.insert(addr as usize, alloc).is_some() {
                panic!("Address {:p} is already allocated.", addr)
            }
        }

        Ok(())
    }

    fn mmap_anon(
        &self,
        len: usize,
        prot: Protections,
        fd: i32,
        offset: i64,
    ) -> Result<*mut u8, MmapError> {
        use std::collections::btree_map::Entry;

        // Check if arguments valid.
        if fd >= 0 {
            return Err(MmapError::NonNegativeFd);
        } else if offset != 0 {
            return Err(MmapError::NonZeroOffset);
        }

        // Do allocation.
        let alloc = match self.alloc(len, prot) {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::OutOfMemory {
                    return Err(MmapError::NoMem(len));
                } else {
                    // We should not hit other error except for out of memory.
                    syserr!("{}", e);
                }
            }
        };

        // Store allocation info.
        let mut allocs = self.allocations.write().unwrap();

        match allocs.entry(alloc.addr as usize) {
            Entry::Occupied(e) => syserr!("address {:p} is already allocated", e.key()),
            Entry::Vacant(e) => Ok(e.insert(alloc).addr),
        }
    }

    fn alloc(&self, len: usize, prot: Protections) -> Result<Alloc, std::io::Error> {
        use self::storage::Memory;

        // Determine how to allocate.
        if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that mean the result of
            // mmap may not aligned correctly. In this case we need to do 2 allocations. The first
            // allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/31411825/1829232
            let len = self.get_reserved_size(len);
            let storage = Memory::new(len)?;

            // Do the second allocation.
            let addr = Self::align_virtual_page(storage.addr());
            let len = len - ((addr as usize) - (storage.addr() as usize));

            storage.commit(addr, len, prot)?;

            Ok(Alloc {
                addr,
                len,
                prot,
                storage: Arc::new(storage),
            })
        } else {
            // If allocation granularity is equal or larger than the virtual page that mean the
            // result of mmap will always aligned correctly.
            let storage = Memory::new(len)?;
            let addr = storage.addr();

            storage.commit(addr, len, prot)?;

            Ok(Alloc {
                addr,
                len,
                prot,
                storage: Arc::new(storage),
            })
        }
    }

    fn get_reserved_size(&self, need: usize) -> usize {
        need + (Self::VIRTUAL_PAGE_SIZE - self.allocation_granularity)
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
            panic!("Failed to get page size: {}.", e);
        }

        (v as usize, v as usize)
    }

    #[cfg(windows)]
    fn get_memory_model() -> (usize, usize) {
        use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
        let mut i: SYSTEM_INFO = unsafe { util::mem::uninit() };

        unsafe { GetSystemInfo(&mut i) };

        (i.dwPageSize as usize, i.dwAllocationGranularity as usize)
    }
}

/// Contains information for an allocation of virtual pages.
struct Alloc {
    addr: *mut u8,
    len: usize,
    prot: Protections,
    storage: Arc<dyn Storage>,
}

impl Alloc {
    fn end(&self) -> *mut u8 {
        unsafe { self.addr.add(self.len) }
    }
}

bitflags! {
    /// Flags to tell what access is possible for the virtual page.
    #[repr(transparent)]
    pub struct Protections: u32 {
        const NONE = 0x00000000;
        const CPU_READ = 0x00000001;
        const CPU_WRITE = 0x00000002;
        const CPU_EXEC = 0x00000004;
        const CPU_MASK = 0x00000007;
        const GPU_EXEC = 0x00000008;
        const GPU_READ = 0x00000010;
        const GPU_WRITE = 0x00000020;
        const GPU_MASK = 0x00000056;
    }
}

impl Protections {
    pub fn contains_unknown(self) -> bool {
        (self.bits >> 6) != 0
    }

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

bitflags! {
    /// Flags for [`MemoryManager::mmap()`].
    #[repr(transparent)]
    pub struct MappingFlags: u32 {
        const MAP_SHARED = 0x00000001;
        const MAP_PRIVATE = 0x00000002;
        const MAP_FIXED = 0x00000010;
        const MAP_INHERIT = 0x00000080;
        const MAP_HASSEMAPHORE = 0x00000200;
        const MAP_STACK = 0x00000400;
        const MAP_NOSYNC = 0x00000800;
        const MAP_ANON = 0x00001000;
        const MAP_NOCORE = 0x00020000;
        const MAP_PREFAULT_READ = 0x00040000;
        const MAP_ALIGNED_SUPER = 0x01000000;
    }
}

impl MappingFlags {
    pub const BEHAVIOR_NONE: u32 = 0;
    pub const BEHAVIOR_SHARED: u32 = 1;
    pub const BEHAVIOR_PRIVATE: u32 = 2;

    pub fn behavior(self) -> u32 {
        self.bits & 3
    }

    /// Gets the value that was supplied with MAP_ALIGNED.
    pub fn alignment(self) -> usize {
        (self.bits >> 24) as usize
    }
}

/// Errors for [`MemoryManager::mmap()`].
#[derive(Debug, Error)]
pub enum MmapError {
    #[error("len is zero")]
    ZeroLen,

    #[error("either MAP_SHARED or MAP_PRIVATE is not specified")]
    NoBehavior,

    #[error("both MAP_SHARED and MAP_PRIVATE is specified")]
    InvalidBehavior,

    #[error("MAP_ANON is specified with non-negative file descriptor")]
    NonNegativeFd,

    #[error("MAP_ANON is specified with non-zero offset")]
    NonZeroOffset,

    #[error("no memory available for {0} bytes")]
    NoMem(usize),
}

impl MmapError {
    pub fn errno(&self) -> i32 {
        match self {
            Self::ZeroLen
            | Self::NoBehavior
            | Self::InvalidBehavior
            | Self::NonNegativeFd
            | Self::NonZeroOffset => EINVAL,
            Self::NoMem(_) => ENOMEM,
        }
    }
}

/// Errors for [`MemoryManager::munmap()`].
#[derive(Debug, Error)]
pub enum MunmapError {
    #[error("addr is not aligned")]
    UnalignedAddr,

    #[error("len is zero")]
    ZeroLen,
}

impl MunmapError {
    pub fn errno(&self) -> i32 {
        match self {
            Self::UnalignedAddr | Self::ZeroLen => EINVAL,
        }
    }
}

/// Represents an error for [`MemoryManager::mprotect()`].
#[derive(Debug, Error)]
pub enum MprotectError {
    #[error("addr is not aligned")]
    UnalignedAddr,

    #[error("len is zero")]
    ZeroLen,

    #[error("invalid addr")]
    InvalidAddr,

    #[error("address {0:p} is not mapped")]
    UnmappedAddr(*const u8),
}

impl MprotectError {
    pub fn errno(&self) -> i32 {
        match self {
            // On Linux InvalidAddr and UnmappedAddr will be ENOMEM. Let's follow FreeBSD man page
            // until there are some games is expect ENOMEM.
            Self::UnalignedAddr | Self::ZeroLen | Self::InvalidAddr | Self::UnmappedAddr(_) => {
                EINVAL
            }
        }
    }
}
