use std::io::Error;
use std::num::NonZero;
use thiserror::Error;

pub use self::builder::*;

mod builder;

/// Represents main memory of the PS4.
///
/// This struct will allocate a 8GB of memory immediately but not commit any parts of it until there
/// is an allocation request. That mean the actual memory usage is not fixed at 8GB but will be
/// dependent on what PS4 applications currently running. If it is a simple game the memory usage
/// might be just a hundred of megabytes.
///
/// RAM always started at address 0.
pub struct Ram {
    mem: *mut u8,
    block_size: NonZero<usize>,
}

impl Ram {
    pub(crate) const SIZE: usize = 1024 * 1024 * 1024 * 8; // 8GB

    /// # Safety
    /// `block_size` must be greater or equal host page size.
    pub unsafe fn new(block_size: NonZero<usize>) -> Result<Self, Error> {
        use std::io::Error;

        // Reserve memory range.
        #[cfg(unix)]
        let mem = {
            use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};
            use std::ptr::null_mut;

            let mem = mmap(
                null_mut(),
                Self::SIZE,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANON,
                -1,
                0,
            );

            if mem == MAP_FAILED {
                return Err(Error::last_os_error());
            }

            mem.cast()
        };

        #[cfg(windows)]
        let mem = {
            use std::ptr::null;
            use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

            let mem = VirtualAlloc(null(), Self::SIZE, MEM_RESERVE, PAGE_NOACCESS);

            if mem.is_null() {
                return Err(Error::last_os_error());
            }

            mem.cast()
        };

        Ok(Self { mem, block_size })
    }

    pub fn host_addr(&self) -> *const u8 {
        self.mem
    }

    pub fn len(&self) -> usize {
        Self::SIZE
    }

    pub fn builder(&mut self) -> RamBuilder {
        RamBuilder::new(self)
    }

    /// # Panics
    /// If `addr` or `len` is not multiply by block size.
    ///
    /// # Safety
    /// This method does not check if `addr` is already allocated. It is undefined behavior if
    /// `addr` + `len` is overlapped with the previous allocation.
    pub unsafe fn alloc(&self, addr: usize, len: NonZero<usize>) -> Result<&mut [u8], RamError> {
        assert_eq!(addr % self.block_size, 0);
        assert_eq!(len.get() % self.block_size, 0);

        if !addr.checked_add(len.get()).is_some_and(|v| v <= Self::SIZE) {
            return Err(RamError::InvalidAddr);
        }

        Self::commit(self.mem.add(addr), len.get())
            .map(|v| std::slice::from_raw_parts_mut(v, len.get()))
            .map_err(RamError::HostFailed)
    }

    /// # Panics
    /// If `addr` or `len` is not multiply by block size.
    ///
    /// # Safety
    /// Accessing the deallocated memory on the host after this will be undefined behavior.
    pub unsafe fn dealloc(&self, addr: usize, len: NonZero<usize>) -> Result<(), RamError> {
        assert_eq!(addr % self.block_size, 0);
        assert_eq!(len.get() % self.block_size, 0);

        if !addr.checked_add(len.get()).is_some_and(|v| v <= Self::SIZE) {
            return Err(RamError::InvalidAddr);
        }

        Self::decommit(self.mem.add(addr), len.get()).map_err(RamError::HostFailed)
    }

    #[cfg(unix)]
    unsafe fn commit(addr: *const u8, len: usize) -> Result<*mut u8, Error> {
        use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_PRIVATE, PROT_READ, PROT_WRITE};

        let ptr = mmap(
            addr.cast_mut().cast(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON | MAP_FIXED,
            -1,
            0,
        );

        if ptr == MAP_FAILED {
            Err(Error::last_os_error())
        } else {
            Ok(ptr.cast())
        }
    }

    #[cfg(windows)]
    unsafe fn commit(addr: *const u8, len: usize) -> Result<*mut u8, Error> {
        use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_READWRITE};

        let ptr = VirtualAlloc(addr.cast(), len, MEM_COMMIT, PAGE_READWRITE);

        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(ptr.cast())
        }
    }

    #[cfg(unix)]
    unsafe fn decommit(addr: *mut u8, len: usize) -> Result<(), Error> {
        use libc::{mprotect, PROT_NONE};

        if mprotect(addr.cast(), len, PROT_NONE) < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    unsafe fn decommit(addr: *mut u8, len: usize) -> Result<(), Error> {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_DECOMMIT};

        if VirtualFree(addr.cast(), len, MEM_DECOMMIT) == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for Ram {
    #[cfg(unix)]
    fn drop(&mut self) {
        use libc::munmap;

        if unsafe { munmap(self.mem.cast(), Self::SIZE) } < 0 {
            panic!(
                "failed to unmap RAM at {:p}: {}",
                self.mem,
                Error::last_os_error()
            );
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(self.mem.cast(), 0, MEM_RELEASE) } == 0 {
            panic!(
                "failed to free RAM at {:p}: {}",
                self.mem,
                Error::last_os_error()
            );
        }
    }
}

unsafe impl Send for Ram {}
unsafe impl Sync for Ram {}

/// Represents an error when an operation on [`Ram`] fails.
#[derive(Debug, Error)]
pub enum RamError {
    #[error("invalid address")]
    InvalidAddr,

    #[error("host failed")]
    HostFailed(#[source] std::io::Error),
}
