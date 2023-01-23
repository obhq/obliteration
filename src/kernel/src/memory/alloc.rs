use super::Protections;
use linked_list_allocator::Heap;
use std::alloc::Layout;
use std::collections::BTreeMap;
use std::ptr::NonNull;
use thiserror::Error;

/// Manage all page allocations.
pub(super) struct Allocator {
    heap: Heap,
    align: usize,
    allocations: BTreeMap<usize, AllocInfo>,
}

impl Allocator {
    pub fn new(ptr: *mut u8, len: usize, align: usize) -> Self {
        Self {
            heap: unsafe { Heap::new(ptr, len) },
            align,
            allocations: BTreeMap::new(),
        }
    }

    pub fn alloc(&mut self, len: usize, prot: Protections) -> Result<NonNull<u8>, AllocError> {
        // Allocate from heap.
        let layout = Layout::from_size_align(len, self.align).unwrap();
        let before = self.heap.used();
        let ptr = match self.heap.allocate_first_fit(layout) {
            Ok(v) => v,
            Err(_) => return Err(AllocError::NoMem),
        };

        // Make sure our allocator did the right thing.
        // FIXME: Remove this check once we sure the allocator always did the right thing.
        let addr = ptr.as_ptr() as usize;
        let len = self.heap.used() - before;

        if addr % self.align != 0 || len % self.align != 0 {
            panic!("The memory allocator returned unaligned allocation.");
        }

        // Change protection.
        if let Err(e) = Self::protect(ptr.as_ptr(), len, prot) {
            // If we are here that mean something seriously wrong like the pointer is not what we
            // expected.
            panic!(
                "Failed to change protection of {:p}:{} to {:?}: {}.",
                ptr.as_ptr(),
                len,
                prot,
                e
            );
        }

        // Store allocation information.
        // FIXME: Remove occupied check once we sure the allocator always did the right thing.
        if self.allocations.range(addr..(addr + len)).count() != 0 {
            panic!("The memory allocator returned an occupied allocation.");
        }

        self.allocations.insert(addr, AllocInfo { len, layout });

        Ok(ptr)
    }

    pub fn free(&mut self, ptr: NonNull<u8>) -> Result<(), FreeError> {
        use std::collections::btree_map::Entry;

        // Get allocation information.
        let addr = ptr.as_ptr() as usize;
        let info = match self.allocations.entry(addr) {
            Entry::Vacant(_) => return Err(FreeError::InvalidPtr),
            Entry::Occupied(e) => e.remove(),
        };

        // Set protection to RW because our allocator might read or write it once we returned this
        // allocation.
        let prot = Protections::CPU_READ | Protections::CPU_WRITE;

        if let Err(e) = Self::protect(ptr.as_ptr(), info.len, prot) {
            panic!(
                "Failed to change protection of {:p}:{} to {:?}: {}.",
                ptr.as_ptr(),
                info.len,
                prot,
                e
            );
        }

        // Dealloc from heap.
        unsafe { self.heap.deallocate(ptr, info.layout) };

        Ok(())
    }

    #[cfg(unix)]
    fn protect(ptr: *mut u8, len: usize, prot: Protections) -> Result<(), std::io::Error> {
        use libc::{mprotect, PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};

        // Build system protection flags.
        let mut sys = PROT_NONE;

        if prot.contains(Protections::CPU_READ) {
            sys |= PROT_READ;
        }

        if prot.contains(Protections::CPU_WRITE) {
            sys |= PROT_WRITE;
        }

        if prot.contains(Protections::CPU_EXEC) {
            sys |= PROT_EXEC;
        }

        // Invoke system API.
        if unsafe { mprotect(ptr as _, len, sys) } < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn protect(ptr: *mut u8, len: usize, prot: Protections) -> Result<(), std::io::Error> {
        use windows_sys::Win32::System::Memory::{
            VirtualProtect, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_NOACCESS,
            PAGE_READONLY, PAGE_READWRITE,
        };

        // Build system protection flags. We cannot use "match" here because we need "|" to do
        // bitwise or.
        let cpu = prot & (Protections::CPU_READ | Protections::CPU_WRITE | Protections::CPU_EXEC);
        let mut sys = if cpu == Protections::CPU_EXEC {
            PAGE_EXECUTE
        } else if cpu == Protections::CPU_EXEC | Protections::CPU_READ {
            PAGE_EXECUTE_READ
        } else if cpu == Protections::CPU_EXEC | Protections::CPU_READ | Protections::CPU_WRITE {
            PAGE_EXECUTE_READWRITE
        } else if cpu == Protections::CPU_READ {
            PAGE_READONLY
        } else if cpu == Protections::CPU_READ | Protections::CPU_WRITE {
            PAGE_READWRITE
        } else if cpu == Protections::CPU_WRITE {
            PAGE_READWRITE
        } else {
            PAGE_NOACCESS
        };

        // Invoke system API.
        if unsafe { VirtualProtect(ptr as _, len, sys, &mut sys) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

/// Contains information for an allocation of virtual pages.
pub(super) struct AllocInfo {
    len: usize, // Actual size of allocation.
    layout: Layout,
}

/// Errors for [`alloc()`][Allocator::alloc].
#[derive(Debug, Error)]
pub enum AllocError {
    #[error("insufficient memory")]
    NoMem,
}

/// Errors for [`free()`][Allocator::free].
#[derive(Debug, Error)]
pub enum FreeError {
    #[error("invalid pointer")]
    InvalidPtr,
}
