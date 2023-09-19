use crate::{GroupGuard, GroupMutex};
use std::fmt::{Display, Formatter};
use std::mem::transmute;
use std::ops::{Deref, DerefMut};

/// RAII structure used to release the shared read access of a lock when dropped.
#[allow(dead_code)]
pub struct GroupMutexReadGuard<'a, T> {
    mtx: &'a GroupMutex<T>,
    lock: GroupGuard<'a>,
}

impl<'a, T> GroupMutexReadGuard<'a, T> {
    pub(crate) fn new(lock: GroupGuard<'a>, mtx: &'a GroupMutex<T>) -> Self {
        Self { lock, mtx }
    }
}

impl<'a, T> Drop for GroupMutexReadGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { *self.mtx.active.get() -= 1 };
    }
}

impl<'a, T> Deref for GroupMutexReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { transmute(self.mtx.value.get()) }
    }
}

impl<'a, T: Display> Display for GroupMutexReadGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl<'a, T: Sync> Sync for GroupMutexReadGuard<'a, T> {}

/// RAII structure used to release the exclusive write access of a lock when dropped.
#[allow(dead_code)]
pub struct GroupMutexWriteGuard<'a, T> {
    mtx: &'a GroupMutex<T>,
    lock: GroupGuard<'a>,
}

impl<'a, T> GroupMutexWriteGuard<'a, T> {
    pub(crate) fn new(lock: GroupGuard<'a>, mtx: &'a GroupMutex<T>) -> Self {
        Self { lock, mtx }
    }
}

impl<'a, T> Drop for GroupMutexWriteGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { *self.mtx.active.get() = 0 };
    }
}

impl<'a, T> Deref for GroupMutexWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { transmute(self.mtx.value.get()) }
    }
}

impl<'a, T> DerefMut for GroupMutexWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute(self.mtx.value.get()) }
    }
}

impl<'a, T: Display> Display for GroupMutexWriteGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl<'a, T: Sync> Sync for GroupMutexWriteGuard<'a, T> {}
