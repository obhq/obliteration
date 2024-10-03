use core::ops::Deref;

/// Provides an access to the value of [Arc](alloc::sync::Arc) on [Context](super::Context) without
/// cloning a reference.
///
/// We need this type because return a reference from [Context](super::Context) require static
/// lifetime, which allow the caller to store it at a global level. Once the value is destroyed that
/// reference will be invalid. We can solve this by return a cloned [Arc](alloc::sync::Arc) but most
/// of the time the caller just want a temporary access to the value, which mean the increment and
/// decrement of a reference is not needed so we invent this type to solve this problem.
///
/// This type work by making itself not [Send] and [Sync], which prevent the caller from storing it
/// at a global level.
pub struct BorrowedArc<T>(*const T); // A pointer make this type automatically !Send and !Sync.

impl<T> BorrowedArc<T> {
    /// # Safety
    /// `v` must be owned by [Arc].
    pub(super) unsafe fn new(v: *const T) -> Self {
        Self(v)
    }

    /// Note that this is an associated function, which means that you have to call it as
    /// `BorrowedArc::as_ptr(v)` instead of `v.as_ptr()`. This is so that there is no conflict with
    /// a method on the inner type.
    pub fn as_ptr(this: &Self) -> *const T {
        this.0
    }
}

impl<T> Deref for BorrowedArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
