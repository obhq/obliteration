// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::builder::*;

use std::collections::BTreeSet;
use std::io::Error;
use std::num::NonZero;
use std::sync::{Mutex, MutexGuard};
use thiserror::Error;

mod builder;

/// Represents main memory of the PS4.
///
/// This struct will immediate reserve a range of memory for its size but not commit any parts of it
/// until there is an allocation request.
///
/// RAM always started at address 0.
pub struct Ram {
    mem: *mut u8,
    len: NonZero<usize>,
    block_size: NonZero<usize>,
    allocated: Mutex<BTreeSet<usize>>,
}

impl Ram {
    /// Panics
    /// If `len` is not multiply by `block_size`.
    ///
    /// # Safety
    /// `block_size` must be greater or equal host page size.
    pub unsafe fn new(len: NonZero<usize>, block_size: NonZero<usize>) -> Result<Self, Error> {
        use std::io::Error;

        assert_eq!(len.get() % block_size, 0);

        // Reserve memory range.
        #[cfg(unix)]
        let mem = {
            use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};
            use std::ptr::null_mut;

            let mem = mmap(
                null_mut(),
                len.get(),
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

            let mem = VirtualAlloc(null(), len.get(), MEM_RESERVE, PAGE_NOACCESS);

            if mem.is_null() {
                return Err(Error::last_os_error());
            }

            mem.cast()
        };

        Ok(Self {
            mem,
            len,
            block_size,
            allocated: Mutex::default(),
        })
    }

    pub fn host_addr(&self) -> *const u8 {
        self.mem
    }

    pub fn len(&self) -> NonZero<usize> {
        self.len
    }

    /// # Panics
    /// If `addr` or `len` is not multiply by block size.
    pub fn alloc(&self, addr: usize, len: NonZero<usize>) -> Result<&mut [u8], RamError> {
        assert_eq!(addr % self.block_size, 0);
        assert_eq!(len.get() % self.block_size, 0);

        // Check if the requested range valid.
        let end = addr.checked_add(len.get()).ok_or(RamError::InvalidAddr)?;

        if end > self.len.get() {
            return Err(RamError::InvalidAddr);
        }

        // Check if the requested range already allocated.
        let mut allocated = self.allocated.lock().unwrap();

        if allocated.range(addr..end).next().is_some() {
            return Err(RamError::InvalidAddr);
        }

        // Commit.
        let start = unsafe { self.mem.add(addr) };
        let mem = unsafe { Self::commit(start, len.get()).map_err(RamError::HostFailed)? };

        // Add range to allocated list.
        for addr in (addr..end).step_by(self.block_size.get()) {
            assert!(allocated.insert(addr));
        }

        Ok(unsafe { std::slice::from_raw_parts_mut(mem, len.get()) })
    }

    /// # Panics
    /// If `addr` or `len` is not multiply by block size.
    ///
    /// # Safety
    /// Accessing the deallocated memory on the host after this will be undefined behavior.
    pub unsafe fn dealloc(&self, addr: usize, len: NonZero<usize>) -> Result<(), RamError> {
        assert_eq!(addr % self.block_size, 0);
        assert_eq!(len.get() % self.block_size, 0);

        // Check if the requested range valid.
        let end = addr.checked_add(len.get()).ok_or(RamError::InvalidAddr)?;

        if end > self.len.get() {
            return Err(RamError::InvalidAddr);
        }

        // Decommit the whole range. No need to check if the range already allocated since it will
        // be no-op anyway.
        let mut allocated = self.allocated.lock().unwrap();

        Self::decommit(self.mem.add(addr), len.get()).map_err(RamError::HostFailed)?;

        for addr in (addr..end).step_by(self.block_size.get()) {
            allocated.remove(&addr);
        }

        Ok(())
    }

    /// Return [`None`] if some part of the requested range is not allocated.
    pub fn lock(&self, addr: usize, len: NonZero<usize>) -> Option<LockedAddr> {
        // Get allocated range.
        let end = addr.checked_add(len.get())?;
        let off = addr % self.block_size;
        let mut next = addr - off;
        let allocated = self.allocated.lock().unwrap();
        let range = allocated.range(next..end);

        // Check if the whole range valid.
        for addr in range {
            if *addr != next {
                return None;
            }

            // This block has been allocated successfully, which mean this addition will never
            // overflow.
            next += self.block_size.get();
        }

        if next < end {
            return None;
        }

        Some(LockedAddr {
            lock: allocated,
            ptr: unsafe { self.mem.add(addr) },
            len,
        })
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

        if unsafe { munmap(self.mem.cast(), self.len.get()) } < 0 {
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

/// RAII struct to prevent a range of memory from deallocated.
pub struct LockedAddr<'a> {
    #[allow(dead_code)]
    lock: MutexGuard<'a, BTreeSet<usize>>,
    ptr: *mut u8,
    len: NonZero<usize>,
}

impl<'a> LockedAddr<'a> {
    /// # Safety
    /// Although the whole memory range guarantee to be valid for the whole lifetime of this struct
    /// but the data is subject to race condition due to the other vCPU may write into this range.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn len(&self) -> NonZero<usize> {
        self.len
    }
}

/// Represents an error when an operation on [`Ram`] fails.
#[derive(Debug, Error)]
pub enum RamError {
    #[error("invalid address")]
    InvalidAddr,

    #[error("host failed")]
    HostFailed(#[source] std::io::Error),
}
