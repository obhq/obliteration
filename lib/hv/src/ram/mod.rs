// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::builder::*;

use super::HvError;
use std::num::NonZero;
use std::ptr::null_mut;

mod builder;
#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod os;

/// RAM of the VM.
///
/// RAM always started at address 0.
pub struct Ram {
    mem: *mut u8,
    len: NonZero<usize>,
}

impl Ram {
    pub(super) fn new(len: NonZero<usize>) -> Result<Self, HvError> {
        // Check page size.
        let host_page_size = self::os::get_page_size().map_err(HvError::GetHostPageSize)?;

        assert!(host_page_size.is_power_of_two());

        if len.get() % host_page_size != 0 {
            return Err(HvError::InvalidRamSize);
        }

        // Allocate pages.
        let mem = self::os::alloc(len).map_err(HvError::CreateRamFailed)?;

        Ok(Self { mem, len })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.mem
    }

    pub fn len(&self) -> NonZero<usize> {
        self.len
    }

    /// Returns null if combination of `addr` and `len` out of allocated memory.
    pub fn slice(&self, addr: usize, len: NonZero<usize>) -> *mut u8 {
        // Check if the requested range valid.
        let end = match addr.checked_add(len.get()) {
            Some(v) => v,
            None => return null_mut(),
        };

        if end > self.len.get() {
            null_mut()
        } else {
            unsafe { self.mem.add(addr) }
        }
    }
}

impl Drop for Ram {
    fn drop(&mut self) {
        unsafe { self::os::free(self.mem, self.len).unwrap() };
    }
}

unsafe impl Send for Ram {}
unsafe impl Sync for Ram {}
