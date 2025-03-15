// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::builder::*;
pub use self::lock::*;
pub(crate) use self::mapper::*;

use super::HvError;
use std::cmp::max;
use std::collections::BTreeMap;
use std::num::NonZero;
use std::sync::{Condvar, Mutex};
use thiserror::Error;

mod builder;
mod lock;
mod mapper;
#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod os;

/// RAM of the VM.
///
/// This struct will immediate reserve a range of memory for its size but not commit any parts of it
/// until there is an allocation request.
///
/// RAM always started at address 0.
pub struct Ram {
    mem: *mut u8,
    len: NonZero<usize>,
    vm_page_size: NonZero<usize>,
    host_page_size: NonZero<usize>,
    allocated: Mutex<BTreeMap<usize, State>>,
    cv: Condvar,
    mapper: Box<dyn RamMapper>,
}

impl Ram {
    pub(super) fn new(
        vm_page_size: NonZero<usize>,
        len: NonZero<usize>,
        mapper: impl RamMapper,
    ) -> Result<Self, HvError> {
        // Check page size.
        let host_page_size = self::os::get_page_size().map_err(HvError::GetHostPageSize)?;

        assert!(host_page_size.is_power_of_two());
        assert!(vm_page_size.is_power_of_two());

        // Check block size.
        let block_size = max(vm_page_size, host_page_size);

        if len.get() % block_size != 0 {
            return Err(HvError::InvalidRamSize);
        }

        // Reserve memory range.
        let mem = self::os::reserve(len).map_err(HvError::CreateRamFailed)?;

        Ok(Self {
            mem,
            len,
            vm_page_size,
            host_page_size,
            allocated: Mutex::default(),
            cv: Condvar::new(),
            mapper: Box::new(mapper),
        })
    }

    pub fn host_addr(&self) -> *const u8 {
        self.mem
    }

    pub fn len(&self) -> NonZero<usize> {
        self.len
    }

    pub fn block_size(&self) -> NonZero<usize> {
        max(self.vm_page_size, self.host_page_size)
    }

    pub fn vm_page_size(&self) -> NonZero<usize> {
        self.vm_page_size
    }

    pub fn host_page_size(&self) -> NonZero<usize> {
        self.host_page_size
    }

    /// # Panics
    /// If `addr` or `len` is not multiply by block size.
    pub fn alloc(&self, addr: usize, len: NonZero<usize>) -> Result<LockedMem, RamError> {
        assert_eq!(addr % self.block_size(), 0);
        assert_eq!(len.get() % self.block_size(), 0);

        // Check if the requested range valid.
        let end = addr.checked_add(len.get()).ok_or(RamError::InvalidAddr)?;

        if end > self.len.get() {
            return Err(RamError::InvalidAddr);
        }

        // Check if the requested range already allocated.
        let mut allocated = self.allocated.lock().unwrap();

        if allocated.range(addr..end).next().is_some() {
            return Err(RamError::AlreadyAllocated);
        }

        // Commit.
        let start = unsafe { self.mem.add(addr) };

        unsafe { self::os::commit(start, len).map_err(RamError::Commit)? };

        if let Err(e) = unsafe { self.mapper.map(start, addr, len) } {
            return Err(RamError::Map(e));
        }

        // Add range to allocated list.
        for addr in (addr..end).step_by(self.block_size().get()) {
            assert!(allocated.insert(addr, State::Locked).is_none());
        }

        // Drop the mutex guard before construct the LockedMem otherwise deadlock is possible.
        drop(allocated);

        Ok(LockedMem::new(self, addr, len))
    }

    /// Attempt to dealloc a range locked by the calling thread will result in a deadlock.
    ///
    /// # Panics
    /// If `addr` or `len` is not multiply by block size.
    pub fn dealloc(&self, mut addr: usize, len: NonZero<usize>) -> Result<(), RamError> {
        assert_eq!(addr % self.block_size(), 0);
        assert_eq!(len.get() % self.block_size(), 0);

        // Check if the requested range valid so we don't end up unmap non-VM memory.
        let end = addr.checked_add(len.get()).ok_or(RamError::InvalidAddr)?;

        if end > self.len.get() {
            return Err(RamError::InvalidAddr);
        }

        // Decommit the whole range.
        let mut allocated = self.allocated.lock().unwrap();

        loop {
            // Get starting address.
            let mut range = allocated.range(addr..end);

            addr = match range.next() {
                Some((&addr, &state)) => {
                    if state == State::Locked {
                        allocated = self.cv.wait(allocated).unwrap();
                        continue;
                    }

                    addr
                }
                None => return Ok(()),
            };

            // Get length of contiguous unlocked region.
            let mut end = addr;
            let mut locked = false;

            for (&addr, &state) in range {
                if state == State::Locked {
                    locked = true;
                    break;
                }

                end = addr;
            }

            end += self.block_size().get();

            // TODO: Unmap this portion from the VM if the OS does not do for us.
            let len = end - addr;

            unsafe { self::os::decommit(self.mem.add(addr), len).map_err(RamError::Decommit)? };

            // Remove decommitted range.
            for addr in (addr..end).step_by(self.block_size().get()) {
                allocated.remove(&addr);
            }

            if !locked {
                return Ok(());
            }

            allocated = self.cv.wait(allocated).unwrap();
            addr = end;
        }
    }

    /// Return [`None`] if some part of the requested range is not allocated.
    ///
    /// Attempt to lock a range that already locked by the calling thread will result in a deadlock.
    pub fn lock(&self, addr: usize, len: NonZero<usize>) -> Option<LockedMem> {
        // Round the address down to block size.
        let end = addr.checked_add(len.get())?;
        let off = addr % self.block_size();
        let begin = addr - off;

        // Lock the whole range.
        let mut next = begin;
        let mut allocated = self.allocated.lock().unwrap();
        let ok = 'top: loop {
            let range = allocated.range_mut(next..end);

            for (&addr, state) in range {
                if addr != next {
                    // There is an unallocated block in the range.
                    break 'top false;
                } else if *state == State::Locked {
                    allocated = self.cv.wait(allocated).unwrap();
                    continue 'top;
                }

                *state = State::Locked;

                // This block has been allocated successfully, which mean this addition will never
                // overflow.
                next += self.block_size().get();
            }

            break true;
        };

        // Check if the whole range has been locked.
        if !ok || next < end {
            for (_, state) in allocated.range_mut(begin..next) {
                *state = State::Unlocked;
            }

            if next != begin {
                self.cv.notify_one();
            }

            return None;
        }

        // Drop the mutex guard before construct the LockedMem otherwise deadlock is possible.
        drop(allocated);

        Some(LockedMem::new(self, addr, len))
    }
}

impl Drop for Ram {
    fn drop(&mut self) {
        // TODO: Unmap this portion from the VM if the OS does not do for us.
        unsafe { self::os::free(self.mem, self.len).unwrap() };
    }
}

unsafe impl Send for Ram {}
unsafe impl Sync for Ram {}

/// State of allocated block.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Locked,
    Unlocked,
}

/// Represents an error when an operation on [`Ram`] fails.
#[derive(Debug, Error)]
pub enum RamError {
    #[error("invalid address")]
    InvalidAddr,

    #[error("already allocated")]
    AlreadyAllocated,

    #[error("couldn't commit the memory")]
    Commit(#[source] std::io::Error),

    #[error("couldn't map the memory to the VM")]
    Map(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("couldn't decommit the memory")]
    Decommit(#[source] std::io::Error),
}
