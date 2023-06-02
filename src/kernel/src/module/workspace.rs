use crate::memory::MemoryManager;
use std::alloc::Layout;
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;
use std::sync::{Mutex, MutexGuard};

/// Additional memory that is contiguous to the module memory.
pub struct ModuleWorkspace<'a> {
    mm: &'a MemoryManager,
    ptr: *mut u8,
    len: usize,
    sealed: Mutex<usize>,
    destructors: Mutex<Vec<Box<dyn FnOnce()>>>,
}

impl<'a> ModuleWorkspace<'a> {
    pub(super) fn new(mm: &'a MemoryManager, ptr: *mut u8, len: usize) -> Self {
        Self {
            mm,
            ptr,
            len,
            sealed: Mutex::new(0),
            destructors: Mutex::default(),
        }
    }

    pub fn push<T: 'static>(&self, value: T) -> Option<*mut T> {
        let mut sealed = self.sealed.lock().unwrap();
        let ptr = unsafe { self.ptr.add(*sealed) };
        let available = self.len - *sealed;

        // Check if the remaining space is enough.
        let layout = Layout::new::<T>();
        let offset = match (ptr as usize) % layout.align() {
            0 => 0,
            v => layout.align() - v,
        };

        if offset + layout.size() > available {
            return None;
        }

        // Move value to the workspace.
        let ptr = unsafe { ptr.add(offset) } as *mut T;

        unsafe { std::ptr::write(ptr, value) };

        self.destructors
            .lock()
            .unwrap()
            .push(Box::new(move || unsafe { std::ptr::drop_in_place(ptr) }));

        // Seal the memory.
        *sealed += offset + layout.size();

        Some(ptr)
    }

    pub fn memory(&self) -> WorkspaceMemory<'_> {
        let sealed = self.sealed.lock().unwrap();

        WorkspaceMemory {
            ptr: unsafe { self.ptr.add(*sealed) },
            len: self.len - *sealed,
            lock: sealed,
        }
    }
}

impl<'a> Drop for ModuleWorkspace<'a> {
    fn drop(&mut self) {
        // Run destructors.
        let destructors = self.destructors.get_mut().unwrap();

        for d in destructors.drain(..).rev() {
            d();
        }

        // Unmap the memory.
        if let Err(e) = self.mm.munmap(self.ptr, self.len) {
            panic!(
                "Failed to unmap {} bytes starting at {:p}: {}.",
                self.len, self.ptr, e
            );
        }
    }
}

/// An exclusive access to unsealed memory of the workspace.
pub struct WorkspaceMemory<'a> {
    lock: MutexGuard<'a, usize>,
    ptr: *mut u8,
    len: usize,
}

impl<'a> WorkspaceMemory<'a> {
    pub fn addr(&self) -> usize {
        self.ptr as _
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub fn seal(mut self, len: usize) {
        if len > self.len {
            panic!("The amount to seal is larger than available space.");
        }

        *self.lock += len;
    }
}

impl<'a, I> Index<I> for WorkspaceMemory<'a>
where
    I: SliceIndex<[u8]>,
{
    type Output = <I as SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a, I> IndexMut<I> for WorkspaceMemory<'a>
where
    I: SliceIndex<[u8]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}
