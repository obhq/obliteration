use super::MemoryManager;

/// Encapsulated one or more virtual pages.
pub struct VPages<'a> {
    mm: &'a MemoryManager,
    ptr: *mut u8,
    len: usize,
}

impl<'a> VPages<'a> {
    pub(super) fn new(mm: &'a MemoryManager, ptr: *mut u8, len: usize) -> Self {
        Self { mm, ptr, len }
    }

    pub fn add(&mut self, offset: usize) {
        self.ptr = unsafe { self.ptr.add(offset) };
        self.len -= offset;
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn into_raw(self) -> *mut u8 {
        let ptr = self.ptr;
        std::mem::forget(self);
        ptr
    }
}

impl<'a> Drop for VPages<'a> {
    fn drop(&mut self) {
        self.mm.munmap(self.ptr, self.len).unwrap();
    }
}
