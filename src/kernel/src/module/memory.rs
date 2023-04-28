use super::Memory;
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

/// Represents the memory of the module in unprotected form.
pub(super) struct UnprotectedMemory<'a>(&'a Memory<'a>);

impl<'a> UnprotectedMemory<'a> {
    pub fn new(unprotected: &'a Memory<'a>) -> Self {
        Self(unprotected)
    }

    pub fn addr(&self) -> usize {
        self.0.addr()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.0.ptr, self.0.len) }
    }
}

impl<'a> Drop for UnprotectedMemory<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.0.protect() {
            // This should never happen because it was succeeded when the memory is initialized.
            panic!("Cannot protect the memory: {e}.");
        }
    }
}

impl<'a, I> Index<I> for UnprotectedMemory<'a>
where
    I: SliceIndex<[u8]>,
{
    type Output = <I as SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a, I> IndexMut<I> for UnprotectedMemory<'a>
where
    I: SliceIndex<[u8]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}
