use crate::errno::{EINVAL, ENOMEM};
use crate::syserr;
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::sync::RwLock;
use thiserror::Error;

/// Manage all paged memory that can be seen by a PS4 app.
pub struct MemoryManager {
    page_size: usize,
    allocation_granularity: usize,
    allocations: RwLock<BTreeMap<usize, AllocInfo>>,
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
        let addr = addr as usize;

        if addr % Self::VIRTUAL_PAGE_SIZE != 0 {
            return Err(MunmapError::UnalignedAddr);
        } else if len == 0 {
            return Err(MunmapError::ZeroLen);
        }

        // Find the first allocation info.
        let mut allocs = self.allocations.write().unwrap();
        let first = match allocs.range(..=addr).next_back() {
            Some(v) => v.1,
            None => return Ok(()),
        };

        // Check if the target address is in the range of first allocation. If it does not that mean
        // the target address is not mapped.
        if (first.end() as usize) <= addr {
            return Ok(());
        }

        // Do unmapping every pages in the range.
        let mut removes: Vec<usize> = Vec::with_capacity(allocs.len());
        let mut splitted: Option<AllocInfo> = None;
        let end = (addr + len) as *mut u8;
        let free = |info: &AllocInfo| -> usize {
            // Unmap the whole region.
            let end = (info.addr as usize) + info.len;
            let addr = match info.original_addr {
                Some(v) => v,
                None => info.addr,
            };

            if let Err(e) = Self::free(addr, end - (addr as usize)) {
                // We should never hit any errors if our code are working correctly.
                syserr!("{}", e);
            }

            info.addr as usize
        };

        // FIXME: In theory it is possible to make this more efficient by remove allocation
        // info in-place. Unfortunately Rust does not provides API to achieve what we want.
        for (&addr, info) in allocs.range((first.addr as usize)..) {
            if end <= info.aligned_addr() {
                // The current region is not in the range to unmap.
                break;
            } else if end < info.end() {
                // The current region is the last region.
                let end = Self::align_virtual_page(end);

                if end == info.end() {
                    // Unmap the whole region.
                    removes.push(free(info));
                } else {
                    // Decommit the partial region.
                    let len = (end as usize) - (info.addr as usize);

                    if let Err(e) = Self::decommit(info.addr, len) {
                        // We should never hit any errors if our code are working correctly.
                        syserr!("{}", e);
                    }

                    // Split the region.
                    removes.push(addr);

                    splitted = Some(AllocInfo {
                        addr: end,
                        len: info.len - len,
                        original_addr: info.original_addr.or(Some(info.addr)),
                    });
                }
            } else {
                // Unmap the whole region.
                removes.push(free(info));
            }
        }

        for addr in removes {
            allocs.remove(&addr);
        }

        if let Some(v) = splitted {
            allocs.insert(v.addr as usize, v);
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
        let info = match self.alloc(len, prot) {
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

        match allocs.entry(info.addr as usize) {
            Entry::Occupied(e) => syserr!("address {:p} is already allocated", e.key()),
            Entry::Vacant(e) => Ok(e.insert(info).aligned_addr()),
        }
    }

    #[cfg(unix)]
    fn alloc(&self, len: usize, prot: Protections) -> Result<AllocInfo, std::io::Error> {
        use libc::{mmap, munmap, MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_PRIVATE, PROT_NONE};
        use std::ptr::null_mut;

        // Determine how to allocate.
        if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that mean the result of
            // mmap may not aligned correctly. In this case we need to do 2 allocations. The first
            // allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/31411825/1829232
            let len = self.get_reserved_size(len);
            let addr = unsafe { mmap(null_mut(), len, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0) };

            if addr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            let info = AllocInfo {
                addr: addr as _,
                len,
                original_addr: None,
            };

            // Do the second allocation.
            let ptr = unsafe {
                mmap(
                    info.aligned_addr() as _,
                    info.aligned_len(),
                    prot.into_host(),
                    MAP_PRIVATE | MAP_ANON | MAP_FIXED,
                    -1,
                    0,
                )
            };

            if ptr == MAP_FAILED {
                let e = std::io::Error::last_os_error();
                unsafe { munmap(info.addr as _, info.len) };
                return Err(e);
            }

            Ok(info)
        } else {
            // If allocation granularity is equal or larger than the virtual page that mean the
            // result of mmap will always aligned correctly.
            let addr = unsafe {
                mmap(
                    null_mut(),
                    len,
                    prot.into_host(),
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                )
            };

            if addr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            Ok(AllocInfo {
                addr: addr as _,
                len,
                original_addr: None,
            })
        }
    }

    #[cfg(windows)]
    fn alloc(&self, len: usize, prot: Protections) -> Result<AllocInfo, std::io::Error> {
        use std::ptr::null;
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS,
        };

        // Determine how to allocate.
        if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that mean the result of
            // VirtualAlloc may not aligned correctly. In this case we need to do 2 allocations. The
            // first allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/7617465/1829232
            let len = self.get_reserved_size(len);
            let addr = unsafe { VirtualAlloc(null(), len, MEM_RESERVE, PAGE_NOACCESS) };

            if addr.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let info = AllocInfo {
                addr: addr as _,
                len,
                original_addr: None,
            };

            // Do the second allocation.
            let ptr = unsafe {
                VirtualAlloc(
                    info.aligned_addr() as _,
                    info.aligned_len(),
                    MEM_COMMIT,
                    prot.into_host(),
                )
            };

            if ptr.is_null() {
                let e = std::io::Error::last_os_error();
                unsafe { VirtualFree(info.addr as _, 0, MEM_RELEASE) };
                return Err(e);
            }

            Ok(info)
        } else {
            // If allocation granularity is equal or larger than the virtual page that mean the
            // result of VirtualAlloc will always aligned correctly.
            let addr =
                unsafe { VirtualAlloc(null(), len, MEM_COMMIT | MEM_RESERVE, prot.into_host()) };

            if addr.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            Ok(AllocInfo {
                addr: addr as _,
                len,
                original_addr: None,
            })
        }
    }

    #[cfg(unix)]
    fn decommit(addr: *mut u8, len: usize) -> Result<(), std::io::Error> {
        use libc::{mprotect, PROT_NONE};

        if unsafe { mprotect(addr as _, len, PROT_NONE) } < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn decommit(addr: *mut u8, len: usize) -> Result<(), std::io::Error> {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_DECOMMIT};

        if unsafe { VirtualFree(addr as _, len, MEM_DECOMMIT) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(unix)]
    fn free(addr: *mut u8, len: usize) -> Result<(), std::io::Error> {
        use libc::munmap;

        if unsafe { munmap(addr as _, len) } < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn free(addr: *mut u8, _len: usize) -> Result<(), std::io::Error> {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(addr as _, 0, MEM_RELEASE) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
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
struct AllocInfo {
    addr: *mut u8,
    len: usize,
    original_addr: Option<*mut u8>,
}

impl AllocInfo {
    fn end(&self) -> *mut u8 {
        unsafe { self.addr.add(self.len) }
    }

    fn aligned_addr(&self) -> *mut u8 {
        MemoryManager::align_virtual_page(self.addr)
    }

    fn aligned_len(&self) -> usize {
        let addr = self.addr as usize;
        let aligned_addr = self.aligned_addr() as usize;

        self.len - (aligned_addr - addr)
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
    pub const BEHAVIOR_BOTH: u32 = 3;

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
