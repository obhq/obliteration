use crate::vmm::{MemoryAddr, VmmError};
use std::io::{Error, ErrorKind};

/// Represents main memory of the PS4.
///
/// This struct will allocate a 8GB of memory immediately but not commit any parts of it until there
/// is an allocation request. That mean the actual memory usage is not fixed at 8GB but will be
/// dependent on what PS4 applications currently running. If it is a simple game the memory usage might be
/// just a hundred of megabytes.
pub struct Ram(*mut u8);

impl Ram {
    pub const ADDR: usize = 0; // It seems like RAM on all system always at address 0.
    pub const SIZE: usize = 1024 * 1024 * 1024 * 8; // 8GB
    pub const VM_PAGE_SIZE: usize = 0x4000;

    pub fn new() -> Result<Self, VmmError> {
        // In theory we can support any page size on the host. The problem is it required a lot of
        // work. It is also unlikely for someone to need this feature because AFAIK the maximum page
        // size on a consumer computer is the same as PS4. With page size the same as PS4 or lower
        // we don't need to keep track allocations here.
        let page_size = Self::get_page_size().map_err(VmmError::GetPageSizeFailed)?;

        if page_size > Self::VM_PAGE_SIZE {
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
                    Self::SIZE,
                    PROT_NONE,
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                )
            };

            if mem == MAP_FAILED {
                return Err(VmmError::CreateRamFailed(Error::last_os_error()));
            }

            mem.cast()
        };

        #[cfg(windows)]
        let mem = {
            use std::ptr::null;
            use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

            let mem = unsafe { VirtualAlloc(null(), Self::SIZE, MEM_RESERVE, PAGE_NOACCESS) };

            if mem.is_null() {
                return Err(VmmError::CreateRamFailed(Error::last_os_error()));
            }

            mem.cast()
        };

        Ok(Self(mem))
    }

    /// # Panics
    /// If `off` or `len` is not multiply by [`Self::VM_PAGE_SIZE`].
    ///
    /// # Safety
    /// This method does not check if `off` is already allocated. It is undefined behavior if
    /// `off` + `len` is overlapped with the previous allocation.
    pub unsafe fn alloc(&self, off: usize, len: usize) -> Result<*mut u8, Error> {
        assert_eq!(off % Self::VM_PAGE_SIZE, 0);
        assert_eq!(len % Self::VM_PAGE_SIZE, 0);

        if off.checked_add(len).unwrap() > Self::SIZE {
            return Err(Error::from(ErrorKind::InvalidInput));
        }

        Self::commit(self.0.add(off), len)
    }

    /// # Panics
    /// If `off` or `len` is not multiply by [`Self::VM_PAGE_SIZE`].
    ///
    /// # Safety
    /// Accessing the deallocated memory on the host will be undefined behavior.
    pub unsafe fn dealloc(&self, off: usize, len: usize) -> Result<(), Error> {
        assert_eq!(off % Self::VM_PAGE_SIZE, 0);
        assert_eq!(len % Self::VM_PAGE_SIZE, 0);

        if off.checked_add(len).unwrap() > Self::SIZE {
            return Err(Error::from(ErrorKind::InvalidInput));
        }

        Self::decommit(self.0.add(off), len)
    }

    #[cfg(unix)]
    fn commit(addr: *const u8, len: usize) -> Result<*mut u8, Error> {
        use libc::{
            mmap, MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE,
        };

        let ptr = unsafe {
            mmap(
                addr.cast_mut().cast(),
                len,
                PROT_EXEC | PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANON | MAP_FIXED,
                -1,
                0,
            )
        };

        if ptr == MAP_FAILED {
            Err(Error::last_os_error())
        } else {
            Ok(ptr.cast())
        }
    }

    #[cfg(windows)]
    fn commit(addr: *const u8, len: usize) -> Result<*mut u8, Error> {
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, MEM_COMMIT, PAGE_EXECUTE_READWRITE,
        };

        let ptr = unsafe { VirtualAlloc(addr.cast(), len, MEM_COMMIT, PAGE_EXECUTE_READWRITE) };

        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(ptr.cast())
        }
    }

    #[cfg(unix)]
    fn decommit(addr: *mut u8, len: usize) -> Result<(), Error> {
        use libc::{mprotect, PROT_NONE};

        if unsafe { mprotect(addr.cast(), len, PROT_NONE) } < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn decommit(addr: *mut u8, len: usize) -> Result<(), Error> {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_DECOMMIT};

        if unsafe { VirtualFree(addr.cast(), len, MEM_DECOMMIT) } == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(unix)]
    fn get_page_size() -> Result<usize, Error> {
        let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

        if v < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(v.try_into().unwrap())
        }
    }

    #[cfg(windows)]
    fn get_page_size() -> Result<usize, Error> {
        use std::mem::zeroed;
        use windows_sys::Win32::System::SystemInformation::GetSystemInfo;
        let mut i = unsafe { zeroed() };

        unsafe { GetSystemInfo(&mut i) };

        Ok(i.dwPageSize.try_into().unwrap())
    }
}

impl Drop for Ram {
    #[cfg(unix)]
    fn drop(&mut self) {
        use libc::munmap;

        if unsafe { munmap(self.0.cast(), Self::SIZE) } < 0 {
            panic!(
                "failed to unmap RAM at {:p}: {}",
                self.0,
                Error::last_os_error()
            );
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(self.0.cast(), 0, MEM_RELEASE) } == 0 {
            panic!(
                "failed to free RAM at {:p}: {}",
                self.0,
                Error::last_os_error()
            );
        }
    }
}

impl MemoryAddr for Ram {
    fn vm_addr(&self) -> usize {
        Self::ADDR
    }

    fn host_addr(&self) -> *const u8 {
        self.0
    }

    fn len(&self) -> usize {
        Self::SIZE
    }
}

unsafe impl Send for Ram {}
unsafe impl Sync for Ram {}
