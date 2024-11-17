use super::GroupGuard;
use core::fmt::{Display, Formatter};
use core::ops::{Deref, DerefMut};

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

impl<'a, T> Drop for GutexWrite<'a, T> {
    fn drop(&mut self) {
        unsafe { *self.active = 0 };
    }
}

impl<'a, T> Deref for GutexWrite<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}

impl<'a, T> DerefMut for GutexWrite<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value }
    }
}

impl<'a, T: Display> Display for GutexWrite<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl<'a, T: Sync> Sync for GutexWrite<'a, T> {}
