// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::builder::*;

use super::HvError;
use std::cmp::min;
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

    /// Returns null if `addr` is not valid.
    ///
    /// This method is safe but using the returned pointer is not. Beware that turning the returned
    /// pointer into a slice **always** trigger UB since the memory can be read or write by any vCPU
    /// at anytime. You need to copy the data to a temporary buffer with [std::ptr::copy()] before
    /// you can access it from safe Rust.
    pub fn slice(&self, addr: usize, len: NonZero<usize>) -> (*mut u8, usize) {
        // Check if the requested range valid.
        let end = addr.saturating_add(len.get());
        let end = min(end, self.len.get());

        match end.checked_sub(addr) {
            Some(len) => unsafe { (self.mem.add(addr), len) },
            None => (null_mut(), 0),
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
