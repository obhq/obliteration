use super::{Ram, State};
use std::cmp::min;
use std::io::Write;
use std::marker::PhantomData;
use std::num::NonZero;

/// RAII struct to prevent a range of memory from deallocated.
pub struct LockedMem<'a> {
    ram: &'a Ram,
    addr: usize,
    len: NonZero<usize>,
}

impl<'a> LockedMem<'a> {
    pub(super) fn new(ram: &'a Ram, addr: usize, len: NonZero<usize>) -> Self {
        Self { ram, addr, len }
    }

    /// # Safety
    /// Although the whole memory range guarantee to be valid for the whole lifetime of this struct
    /// but the data is subject to race condition due to the other vCPU may write into this range.
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.ram.mem.add(self.addr) }
    }

    /// # Safety
    /// Although the whole memory range guarantee to be valid for the whole lifetime of this struct
    /// but the data is subject to race condition due to the other vCPU may write into this range.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.ram.mem.add(self.addr) }
    }

    pub fn len(&self) -> NonZero<usize> {
        self.len
    }

    /// Returns `val` if the space at `off` is not enough for it.
    pub fn put<T>(&mut self, off: usize, val: T) -> Option<T> {
        // Check if the value can fit within a locked range.
        if off
            .checked_add(size_of::<T>())
            .is_none_or(|end| end > self.len.get())
        {
            return Some(val);
        }

        // Write.
        let ptr = unsafe { self.as_mut_ptr().add(off) };

        unsafe { ptr.cast::<T>().write_unaligned(val) };

        None
    }

    pub fn writer(&mut self, off: usize, len: Option<usize>) -> Option<impl Write + '_> {
        // Get end offset.
        let end = match len {
            Some(len) => off.checked_add(len).filter(|&v| v <= self.len.get())?,
            None => {
                if off > self.len.get() {
                    return None;
                }

                self.len.get()
            }
        };

        // Construct Writer.
        let base = self.as_mut_ptr();
        let ptr = unsafe { base.add(off) };
        let end = unsafe { base.add(end) };

        Some(Writer {
            ptr,
            end,
            phantom: PhantomData,
        })
    }
}

impl Drop for LockedMem<'_> {
    fn drop(&mut self) {
        // Round the address down to block size.
        let off = self.addr % self.ram.block_size;
        let begin = self.addr - off;
        let end = self.addr + self.len.get();

        // Unlock the range.
        let mut allocated = self.ram.allocated.lock().unwrap();

        for (_, s) in allocated.range_mut(begin..end) {
            *s = State::Unlocked;
        }

        self.ram.cv.notify_one();
    }
}

/// Provides [`Write`] implementation to write a region of [`LockedMem`].
struct Writer<'a> {
    ptr: *mut u8,
    end: *const u8,
    phantom: PhantomData<&'a ()>,
}

impl Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let available = unsafe { self.end.offset_from(self.ptr).try_into().unwrap() };
        let len = min(buf.len(), available);

        // SAFETY: We mutable borrow the LockedMem so buf is never from the same LockedMem.
        unsafe { self.ptr.copy_from_nonoverlapping(buf.as_ptr(), len) };
        unsafe { self.ptr = self.ptr.add(len) };

        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
