use crate::errno::{EINVAL, ENOMEM};
use crate::syserr;
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::os::raw::c_void;
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
        todo!()
    }

    fn mmap_anon(
        &self,
        len: usize,
        prot: Protections,
        fd: i32,
        offset: i64,
    ) -> Result<*mut u8, MmapError> {
        // Check if arguments valid.
        if fd >= 0 {
            return Err(MmapError::NonNegativeFd);
        } else if offset != 0 {
            return Err(MmapError::NonZeroOffset);
        }

        // Do allocation.
        let (ptr, info) = match self.alloc(len, prot) {
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

        if allocs.insert(ptr as usize, info).is_some() {
            syserr!("address {:p} is already allocated", ptr);
        }

        Ok(ptr)
    }

    #[cfg(unix)]
    fn alloc(&self, len: usize, prot: Protections) -> Result<(*mut u8, AllocInfo), std::io::Error> {
        use libc::{mmap, munmap, MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_PRIVATE, PROT_NONE};
        use std::ptr::null_mut;

        // Determine how to allocate.
        if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that mean the result of
            // mmap may not aligned correctly. In this case we need to do 2 allocations. The first
            // allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/31411825/1829232
            let reserved_len = self.get_reserved_size(len);
            let reserved_addr = unsafe {
                mmap(
                    null_mut(),
                    reserved_len,
                    PROT_NONE,
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                )
            };

            if reserved_addr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            // Do the second allocation.
            let ptr = unsafe {
                mmap(
                    Self::align_virtual_page(reserved_addr),
                    len,
                    prot.into_host(),
                    MAP_PRIVATE | MAP_ANON | MAP_FIXED,
                    -1,
                    0,
                )
            };

            if ptr == MAP_FAILED {
                let e = std::io::Error::last_os_error();
                unsafe { munmap(reserved_addr, reserved_len) };
                return Err(e);
            }

            // Build allocation info.
            let info = AllocInfo {
                reserved_addr: reserved_addr as _,
                reserved_len,
            };

            Ok((ptr as _, info))
        } else {
            // If allocation granularity is equal or larger than the virtual page that mean the
            // result of mmap will always aligned correctly.
            let ptr = unsafe {
                mmap(
                    null_mut(),
                    len,
                    prot.into_host(),
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                )
            };

            if ptr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            // Build allocation info.
            let info = AllocInfo {
                reserved_addr: null_mut(),
                reserved_len: 0,
            };

            Ok((ptr as _, info))
        }
    }

    #[cfg(windows)]
    fn alloc(&self, len: usize, prot: Protections) -> Result<(*mut u8, AllocInfo), std::io::Error> {
        use std::ptr::{null, null_mut};
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS,
        };

        // Determine how to allocate.
        if self.allocation_granularity < Self::VIRTUAL_PAGE_SIZE {
            // If allocation granularity is smaller than the virtual page that mean the result of
            // VirtualAlloc may not aligned correctly. In this case we need to do 2 allocations. The
            // first allocation will be large enough for a second allocation with fixed address.
            // The whole idea is coming from: https://stackoverflow.com/a/7617465/1829232
            let reserved_len = self.get_reserved_size(len);
            let reserved_addr =
                unsafe { VirtualAlloc(null(), reserved_len, MEM_RESERVE, PAGE_NOACCESS) };

            if reserved_addr.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            // Do the second allocation.
            let ptr = unsafe {
                VirtualAlloc(
                    Self::align_virtual_page(reserved_addr),
                    len,
                    MEM_COMMIT,
                    prot.into_host(),
                )
            };

            if ptr.is_null() {
                let e = std::io::Error::last_os_error();
                unsafe { VirtualFree(reserved_addr, 0, MEM_RELEASE) };
                return Err(e);
            }

            // Build allocation info.
            let info = AllocInfo {
                reserved_addr: reserved_addr as _,
                reserved_len,
            };

            Ok((ptr as _, info))
        } else {
            // If allocation granularity is equal or larger than the virtual page that mean the
            // result of VirtualAlloc will always aligned correctly.
            let ptr =
                unsafe { VirtualAlloc(null(), len, MEM_COMMIT | MEM_RESERVE, prot.into_host()) };

            if ptr.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            // Build allocation info.
            let info = AllocInfo {
                reserved_addr: null_mut(),
                reserved_len: 0,
            };

            Ok((ptr as _, info))
        }
    }

    fn get_reserved_size(&self, need: usize) -> usize {
        need + (Self::VIRTUAL_PAGE_SIZE - self.allocation_granularity)
    }

    fn align_virtual_page(ptr: *mut c_void) -> *mut c_void {
        let addr = ptr as usize;
        let aligned = match addr % Self::VIRTUAL_PAGE_SIZE {
            0 => addr,
            v => addr + (Self::VIRTUAL_PAGE_SIZE - v),
        };

        aligned as *mut c_void
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
        let mut i: SYSTEM_INFO = util::mem::uninit();

        unsafe { GetSystemInfo(&mut i) };

        (i.dwPageSize as usize, i.dwAllocationGranularity as usize)
    }
}

/// Contains information for an allocation of virtual pages.
struct AllocInfo {
    reserved_addr: *mut u8,
    reserved_len: usize,
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
        } else if cpu == Self::CPU_READ | Self::CPU_WRITE {
            PAGE_READWRITE
        } else if cpu == Self::CPU_WRITE {
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
pub enum MunmapError {}
