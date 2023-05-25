use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;
use std::sync::{Mutex, MutexGuard};

/// Additional memory that is contiguous to the module memory.
pub struct ModuleWorkspace {
    ptr: *mut u8,
    len: usize,
    sealed: Mutex<usize>,
}

impl ModuleWorkspace {
    pub(super) fn new(ptr: *mut u8, len: usize) -> Self {
        Self {
            ptr,
            len,
            sealed: Mutex::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn lock(&self) -> UnsealedWorkspace<'_> {
        let sealed = self.sealed.lock().unwrap();

        UnsealedWorkspace {
            ptr: unsafe { self.ptr.add(*sealed) },
            len: self.len - *sealed,
            lock: sealed,
        }
    }
}

/// An exclusive access to unsealed memory of the workspace.
pub struct UnsealedWorkspace<'a> {
    lock: MutexGuard<'a, usize>,
    ptr: *mut u8,
    len: usize,
}

impl<'a> UnsealedWorkspace<'a> {
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

impl<'a, I> Index<I> for UnsealedWorkspace<'a>
where
    I: SliceIndex<[u8]>,
{
    type Output = <I as SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a, I> IndexMut<I> for UnsealedWorkspace<'a>
where
    I: SliceIndex<[u8]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}
