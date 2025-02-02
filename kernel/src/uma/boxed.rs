use core::ops::Deref;

/// Encapsulates an object allocated from a UMA zone.
pub struct UmaBox<T: ?Sized>(*mut T);

impl<T: ?Sized> Deref for UmaBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

unsafe impl<T: Send + ?Sized> Send for UmaBox<T> {}
unsafe impl<T: Sync + ?Sized> Sync for UmaBox<T> {}
