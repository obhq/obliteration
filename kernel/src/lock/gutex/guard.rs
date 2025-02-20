use super::GroupGuard;
use core::fmt::{Display, Formatter};
use core::ops::{Deref, DerefMut};

/// RAII structure used to release the shared read access of a lock when dropped.
pub struct GutexRead<'a, T> {
    #[allow(dead_code)] // active and value fields is protected by this lock.
    lock: GroupGuard<'a>,
    active: *mut usize,
    value: *const T,
}

impl<'a, T> GutexRead<'a, T> {
    /// # Safety
    /// `active` and `value` must be protected by `lock`.
    pub(super) fn new(lock: GroupGuard<'a>, active: *mut usize, value: *const T) -> Self {
        Self {
            lock,
            active,
            value,
        }
    }
}

impl<T> Drop for GutexRead<'_, T> {
    fn drop(&mut self) {
        unsafe { *self.active -= 1 };
    }
}

impl<T> Deref for GutexRead<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}

impl<T: Display> Display for GutexRead<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl<T: Sync> Sync for GutexRead<'_, T> {}

/// RAII structure used to release the exclusive write access of a lock when dropped.
pub struct GutexWrite<'a, T> {
    #[allow(dead_code)] // active and value fields is protected by this lock.
    lock: GroupGuard<'a>,
    active: *mut usize,
    value: *mut T,
}

impl<'a, T> GutexWrite<'a, T> {
    /// # Safety
    /// `active` and `value` must be protected by `lock`.
    pub(super) unsafe fn new(lock: GroupGuard<'a>, active: *mut usize, value: *mut T) -> Self {
        Self {
            active,
            value,
            lock,
        }
    }
}

impl<T> Drop for GutexWrite<'_, T> {
    fn drop(&mut self) {
        unsafe { *self.active = 0 };
    }
}

impl<T> Deref for GutexWrite<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}

impl<T> DerefMut for GutexWrite<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value }
    }
}

impl<T: Display> Display for GutexWrite<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl<T: Sync> Sync for GutexWrite<'_, T> {}
