use core::ops::Deref;
use core::ptr::NonNull;

/// Strong reference to reference-counted memory block.
///
/// The main different from [alloc::sync::Arc] is this type store the number of references alongside
/// the data.
pub struct Strong<T: RefCnt + ?Sized>(NonNull<T>);

impl<T: RefCnt + ?Sized> Strong<T> {
    /// # Safety
    /// `v` cannot be null and must point to initialized value.
    pub unsafe fn new(v: *const T) -> Self {
        unsafe { (*v).increase_ref() };

        Self(unsafe { NonNull::new_unchecked(v.cast_mut()) })
    }

    pub fn as_ptr(this: &Self) -> NonNull<T> {
        this.0
    }
}

impl<T: RefCnt + ?Sized> Drop for Strong<T> {
    fn drop(&mut self) {
        let v = self.0.as_ptr();
        let r = unsafe { (*v).decrease_ref() };

        if r == 1 {
            unsafe { core::ptr::drop_in_place(v) };
        }
    }
}

impl<T: RefCnt + ?Sized> Deref for Strong<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T: RefCnt + ?Sized> Clone for Strong<T> {
    fn clone(&self) -> Self {
        let v = self.0.as_ptr();

        unsafe { (*v).increase_ref() };

        Self(unsafe { NonNull::new_unchecked(v) })
    }
}

unsafe impl<T: RefCnt + Send + ?Sized> Send for Strong<T> {}
unsafe impl<T: RefCnt + Sync + ?Sized> Sync for Strong<T> {}

/// Provides methods to increase/decrease a strong reference to reference-counted mmemory block.
///
/// # Safety
/// The number of strong references store on the memory can only modified by [Self::increase_ref()]
/// and [Self::decrease_ref()]. The initial value must be zero.
pub unsafe trait RefCnt {
    /// Increments the strong reference count on the memory.
    ///
    /// # Panics
    /// If reference count already at [usize::MAX].
    fn increase_ref(&self);

    /// Decrements the strong reference count on the memory and returns number of references before
    /// the decreasement.
    fn decrease_ref(&self) -> usize;
}
