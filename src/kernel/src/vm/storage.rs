use super::Protections;
use std::ffi::CStr;
use std::fmt::Debug;
use std::io::Error;

/// Represents a storage for [`super::Alloc`].
///
/// Multiple [`super::Alloc`] can share a single storage.
pub(super) trait Storage: Debug {
    fn addr(&self) -> *mut u8;
    fn decommit(&self, addr: *mut u8, len: usize) -> Result<(), Error>;
    fn protect(&self, addr: *mut u8, len: usize, prot: Protections) -> Result<(), Error>;
    fn set_name(&self, addr: *mut u8, len: usize, name: &CStr) -> Result<(), Error>;
}

/// An implementation of [`Storage`] backed by the memory.
#[derive(Debug)]
pub(super) struct Memory {
    addr: *mut u8,
    len: usize,
}

impl Memory {
    #[cfg(unix)]
    pub fn new(addr: usize, len: usize) -> Result<Self, Error> {
        use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};

        let addr = unsafe { mmap(addr as _, len, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0) };

        if addr == MAP_FAILED {
            return Err(Error::last_os_error());
        }

        Ok(Self {
            addr: addr as _,
            len,
        })
    }

    #[cfg(windows)]
    pub fn new(addr: usize, len: usize) -> Result<Self, Error> {
        use std::ptr::null;
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_INVALID_ADDRESS};
        use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

        let ptr = unsafe { VirtualAlloc(addr as _, len, MEM_RESERVE, PAGE_NOACCESS) };

        if !ptr.is_null() {
            return Ok(Self {
                addr: ptr as _,
                len,
            });
        } else if addr == 0 || unsafe { GetLastError() } != ERROR_INVALID_ADDRESS {
            return Err(Error::last_os_error());
        }

        // Windows will fail with ERROR_INVALID_ADDRESS instead of returning the allocated memory
        // somewhere else if the requested address cannot be reserved.
        let ptr = unsafe { VirtualAlloc(null(), len, MEM_RESERVE, PAGE_NOACCESS) };

        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(Self {
                addr: ptr as _,
                len,
            })
        }
    }

    #[cfg(unix)]
    pub fn commit(&self, addr: *const u8, len: usize, prot: Protections) -> Result<(), Error> {
        use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_PRIVATE};

        let ptr = unsafe {
            mmap(
                addr as _,
                len,
                prot.into_host(),
                MAP_PRIVATE | MAP_ANON | MAP_FIXED,
                -1,
                0,
            )
        };

        if ptr == MAP_FAILED {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    pub fn commit(&self, addr: *const u8, len: usize, prot: Protections) -> Result<(), Error> {
        use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT};

        let ptr = unsafe { VirtualAlloc(addr as _, len, MEM_COMMIT, prot.into_host()) };

        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Storage for Memory {
    fn addr(&self) -> *mut u8 {
        self.addr
    }

    #[cfg(unix)]
    fn decommit(&self, addr: *mut u8, len: usize) -> Result<(), Error> {
        use libc::{mprotect, PROT_NONE};

        if unsafe { mprotect(addr as _, len, PROT_NONE) } < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn decommit(&self, addr: *mut u8, len: usize) -> Result<(), Error> {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_DECOMMIT};

        if unsafe { VirtualFree(addr as _, len, MEM_DECOMMIT) } == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(unix)]
    fn protect(&self, addr: *mut u8, len: usize, prot: Protections) -> Result<(), Error> {
        use libc::mprotect;

        if unsafe { mprotect(addr as _, len, prot.into_host()) } < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn protect(&self, addr: *mut u8, len: usize, prot: Protections) -> Result<(), Error> {
        use windows_sys::Win32::System::Memory::VirtualProtect;

        let mut old = 0;

        if unsafe { VirtualProtect(addr as _, len, prot.into_host(), &mut old) } == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(target_os = "linux")]
    fn set_name(&self, addr: *mut u8, len: usize, name: &CStr) -> Result<(), Error> {
        use libc::{prctl, PR_SET_VMA, PR_SET_VMA_ANON_NAME};

        if unsafe { prctl(PR_SET_VMA, PR_SET_VMA_ANON_NAME, addr, len, name.as_ptr()) } < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn set_name(&self, _: *mut u8, _: usize, _: &CStr) -> Result<(), Error> {
        Ok(())
    }
}

impl Drop for Memory {
    #[cfg(unix)]
    fn drop(&mut self) {
        use libc::munmap;

        if unsafe { munmap(self.addr as _, self.len) } < 0 {
            let e = Error::last_os_error();
            panic!("Failed to unmap {:p}:{}: {}.", self.addr, self.len, e);
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(self.addr as _, 0, MEM_RELEASE) } == 0 {
            let e = Error::last_os_error();
            panic!("Failed to free {:p}: {}.", self.addr, e);
        }
    }
}
