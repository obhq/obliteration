use super::Protections;
use std::fmt::Debug;

/// Represents a storage for [`super::Alloc`].
///
/// Multiple [`super::Alloc`] can share a single storage.
pub(super) trait Storage: Debug {
    fn addr(&self) -> *mut u8;
    fn decommit(&self, addr: *mut u8, len: usize) -> Result<(), std::io::Error>;
    fn protect(&self, addr: *mut u8, len: usize, prot: Protections) -> Result<(), std::io::Error>;
}

/// An implementation of [`Storage`] backed by the memory.
#[derive(Debug)]
pub(super) struct Memory {
    addr: *mut u8,
    len: usize,
}

impl Memory {
    #[cfg(unix)]
    pub fn new(len: usize) -> Result<Self, std::io::Error> {
        use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_NONE};
        use std::ptr::null_mut;

        let addr = unsafe { mmap(null_mut(), len, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0) };

        if addr == MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            addr: addr as _,
            len,
        })
    }

    #[cfg(windows)]
    pub fn new(len: usize) -> Result<Self, std::io::Error> {
        use std::ptr::null;
        use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

        let addr = unsafe { VirtualAlloc(null(), len, MEM_RESERVE, PAGE_NOACCESS) };

        if addr.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            addr: addr as _,
            len,
        })
    }

    #[cfg(unix)]
    pub fn commit(
        &self,
        addr: *const u8,
        len: usize,
        prot: Protections,
    ) -> Result<(), std::io::Error> {
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
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    pub fn commit(
        &self,
        addr: *const u8,
        len: usize,
        prot: Protections,
    ) -> Result<(), std::io::Error> {
        use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT};

        let ptr = unsafe { VirtualAlloc(addr as _, len, MEM_COMMIT, prot.into_host()) };

        if ptr.is_null() {
            Err(std::io::Error::last_os_error())
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
    fn decommit(&self, addr: *mut u8, len: usize) -> Result<(), std::io::Error> {
        use libc::{mprotect, PROT_NONE};

        if unsafe { mprotect(addr as _, len, PROT_NONE) } < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn decommit(&self, addr: *mut u8, len: usize) -> Result<(), std::io::Error> {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_DECOMMIT};

        if unsafe { VirtualFree(addr as _, len, MEM_DECOMMIT) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(unix)]
    fn protect(&self, addr: *mut u8, len: usize, prot: Protections) -> Result<(), std::io::Error> {
        use libc::mprotect;

        if unsafe { mprotect(addr as _, len, prot.into_host()) } < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn protect(&self, addr: *mut u8, len: usize, prot: Protections) -> Result<(), std::io::Error> {
        use windows_sys::Win32::System::Memory::VirtualProtect;

        let mut old = 0;

        if unsafe { VirtualProtect(addr as _, len, prot.into_host(), &mut old) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for Memory {
    #[cfg(unix)]
    fn drop(&mut self) {
        use libc::munmap;

        if unsafe { munmap(self.addr as _, self.len) } < 0 {
            let e = std::io::Error::last_os_error();
            panic!("Failed to unmap {:p}:{}: {}.", self.addr, self.len, e);
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(self.addr as _, 0, MEM_RELEASE) } == 0 {
            let e = std::io::Error::last_os_error();
            panic!("Failed to free {:p}: {}.", self.addr, e);
        }
    }
}
