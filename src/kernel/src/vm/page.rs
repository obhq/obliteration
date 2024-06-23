use super::VmSpace;

/// Encapsulated one or more virtual pages.
pub struct VPages<'a> {
    mm: &'a VmSpace,
    ptr: *mut u8,
    len: usize,
}

impl<'a> VPages<'a> {
    pub(super) fn new(mm: &'a VmSpace, ptr: *mut u8, len: usize) -> Self {
        Self { mm, ptr, len }
    }

    pub fn addr(&self) -> usize {
        self.ptr as _
    }

    pub fn end(&self) -> *const u8 {
        unsafe { self.ptr.add(self.len) }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
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
