use crate::errno::EINVAL;
use crate::syserr;
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::sync::RwLock;
use thiserror::Error;

/// Manage all paged memory that can be seen by a PS4 app.
pub struct MemoryManager {
    ptr: *mut u8,
    len: usize,
    page_size: usize,
    allocations: RwLock<BTreeMap<usize, AllocInfo>>,
}

impl MemoryManager {
    /// Size of a memory page on PS4.
    pub const VIRTUAL_PAGE_SIZE: usize = 0x4000;

    pub(super) fn new(len: usize) -> Result<Self, NewError> {
        if len == 0 {
            return Err(NewError::ZeroLen);
        }

        // Check if page size on the host is supported.
        let page_size = Self::get_page_size();

        if page_size > Self::VIRTUAL_PAGE_SIZE || (Self::VIRTUAL_PAGE_SIZE % page_size) != 0 {
            panic!("Your system are using unsupported page size.");
        }

        // Round-up the size of total memory.
        let len = match len % page_size {
            0 => len,
            v => len + (page_size - v),
        };

        // Allocate a memory that ack as a total available memory for PS4.
        let ptr = match Self::alloc_pages(len) {
            Ok(v) => v,
            Err(e) => return Err(NewError::AllocFailed(len, e)),
        };

        Ok(Self {
            ptr,
            len,
            page_size,
            allocations: RwLock::new(BTreeMap::new()),
        })
    }

    /// Gets size of page on the host system.
    pub fn page_size(&self) -> usize {
        self.page_size
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

        // Check mapping source.
        if flags.contains(MappingFlags::MAP_ANON) {
            if fd >= 0 {
                return Err(MmapError::NonNegativeFd);
            } else if offset != 0 {
                return Err(MmapError::NonZeroOffset);
            }
        } else {
            syserr!("non-anonymous mapping is not supported yet");
        }

        todo!()
    }

    pub fn munmap(&self, addr: *mut u8, len: usize) -> Result<(), MunmapError> {
        todo!()
    }

    #[cfg(unix)]
    fn alloc_pages(len: usize) -> Result<*mut u8, std::io::Error> {
        use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};
        use std::ptr::null_mut;

        let ptr = unsafe { mmap(null_mut(), len, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0) };

        if ptr == MAP_FAILED {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ptr as _)
        }
    }

    #[cfg(windows)]
    fn alloc_pages(len: usize) -> Result<*mut u8, std::io::Error> {
        use std::ptr::null;
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_NOACCESS,
        };

        let ptr = unsafe { VirtualAlloc(null(), len, MEM_COMMIT | MEM_RESERVE, PAGE_NOACCESS) };

        if ptr.is_null() {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ptr as _)
        }
    }

    #[cfg(unix)]
    fn get_page_size() -> usize {
        let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

        if v < 0 {
            let e = std::io::Error::last_os_error();
            panic!("Failed to get page size: {}.", e);
        }

        v as usize
    }

    #[cfg(windows)]
    fn get_page_size() -> usize {
        use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
        let mut i: SYSTEM_INFO = util::mem::uninit();

        unsafe { GetSystemInfo(&mut i) };

        i.dwPageSize as usize
    }
}

impl Drop for MemoryManager {
    #[cfg(unix)]
    fn drop(&mut self) {
        use libc::munmap;

        if unsafe { munmap(self.ptr as _, self.len) } < 0 {
            let e = std::io::Error::last_os_error();

            panic!(
                "Failed to unmap {} bytes starting at {:p}: {}",
                self.len, self.ptr, e
            );
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(self.ptr as _, 0, MEM_RELEASE) } == 0 {
            let e = std::io::Error::last_os_error();

            panic!(
                "Failed to free {} bytes starting at {:p}: {}",
                self.len, self.ptr, e
            );
        }
    }
}

/// Contains information for an allocation of virtual pages.
struct AllocInfo {
    len: usize,
    prot: Protections,
}

bitflags! {
    /// Flags to tell what access is possible for the virtual page.
    #[repr(transparent)]
    pub struct Protections: u8 {
        const NONE = 0;
        const CPU_READ = 1;
        const CPU_WRITE = 2;
        const CPU_EXEC = 4;
        const GPU_EXEC = 8;
        const GPU_READ = 16;
        const GPU_WRITE = 32;
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

/// Error for [`MemoryManager::new()`].
#[derive(Debug, Error)]
pub enum NewError {
    #[error("len is zero")]
    ZeroLen,

    #[error("cannot allocate {0} bytes of memory")]
    AllocFailed(usize, std::io::Error),
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
}

impl MmapError {
    pub fn errno(&self) -> i32 {
        match self {
            Self::ZeroLen
            | Self::NoBehavior
            | Self::InvalidBehavior
            | Self::NonNegativeFd
            | Self::NonZeroOffset => EINVAL,
        }
    }
}

/// Errors for [`MemoryManager::munmap()`].
#[derive(Debug, Error)]
pub enum MunmapError {}
