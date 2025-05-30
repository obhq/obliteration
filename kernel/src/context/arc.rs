use alloc::sync::Arc;
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
#[repr(transparent)]
pub struct BorrowedArc<T>(*const T); // A pointer make this type automatically !Send and !Sync.

impl<T> BorrowedArc<T> {
    /// # Safety
    /// `v` must be owned by [Arc](alloc::sync::Arc) if not null.
    pub(super) const unsafe fn new(v: *const T) -> Option<Self> {
        if v.is_null() { None } else { Some(Self(v)) }
    }

    /// # Safety
    /// `v` must be owned by [Arc](alloc::sync::Arc).
    pub(super) const unsafe fn from_non_null(v: *const T) -> Self {
        Self(v)
    }

    /// Note that this is an associated function, which means that you have to call it as
    /// `BorrowedArc::as_ptr(v)` instead of `v.as_ptr()`. This is so that there is no conflict with
    /// a method on the inner type.
    pub fn as_ptr(this: &Self) -> *const T {
        this.0
    }

    pub fn into_owned(self) -> Arc<T> {
        // SAFETY: This is safe because the requirement of new() and from_non_null().
        unsafe { Arc::increment_strong_count(self.0) };
        unsafe { Arc::from_raw(self.0) }
    }
}

impl<T> Clone for BorrowedArc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for BorrowedArc<T> {}

impl<T> Deref for BorrowedArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
