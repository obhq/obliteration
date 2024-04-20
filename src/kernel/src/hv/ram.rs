use super::MemoryAddr;
use std::io::Error;

/// Represents main memory of the PS4.
///
/// This struct will allocate a 8GB of memory immediately but not commit any parts of it until there
/// is an allocation request. That mean the actual memory usage is not fixed at 8GB but will be
/// dependent on what PS4 applications currently running. If it is a simple game the memory usage might be
/// just a hundred of megabytes.
pub struct Ram {
    addr: usize,
    mem: *mut u8,
}

impl Ram {
    pub const SIZE: usize = 1024 * 1024 * 1024 * 8; // 8GB

    pub fn new(addr: usize) -> Result<Self, Error> {
        // Reserve a memory range on *nix.
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
                return Err(Error::last_os_error());
            }

            mem.cast()
        };

        // Reserve a memory range on Windows.
        #[cfg(windows)]
        let mem = {
            use std::ptr::null;
            use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_RESERVE, PAGE_NOACCESS};

            let mem = unsafe { VirtualAlloc(null(), Self::SIZE, MEM_RESERVE, PAGE_NOACCESS) };

            if mem.is_null() {
                return Err(Error::last_os_error());
            }

            mem.cast()
        };

        Ok(Self { addr, mem })
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

impl MemoryAddr for Ram {
    fn vm_addr(&self) -> usize {
        self.addr
    }

    fn host_addr(&self) -> *mut () {
        self.mem.cast()
    }

    fn len(&self) -> usize {
        Self::SIZE
    }
}

unsafe impl Send for Ram {}
unsafe impl Sync for Ram {}
