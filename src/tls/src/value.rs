use std::marker::PhantomData;
use std::ops::Deref;

/// Encapsulate a value owned by a thread.
///
/// This object cannot be send across the thread because the encapsulated value will be destroyed
/// when the thread that owned it exited.
pub struct Local<'a, T> {
    value: *const T,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> Local<'a, T> {
    pub(crate) fn new(value: *const T) -> Self {
        Self {
            value,
            phantom: PhantomData,
        }
    }
}

impl<'a, T> Deref for Local<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}

unsafe impl<'a, T: Sync> Sync for Local<'a, T> {}
