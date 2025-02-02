use core::ops::Deref;

/// Encapsulates an object allocated from a UMA zone.
pub struct UmaBox<T: ?Sized>(*mut T);

impl<T: ?Sized> Deref for UmaBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
