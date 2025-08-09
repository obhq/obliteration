use super::MTX_UNOWNED;
use crate::context::{BorrowedArc, current_thread};
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Implementation of `mtx` structure.
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    owning: AtomicUsize,          // mtx_lock
    phantom: PhantomData<Rc<()>>, // For !Send and !Sync.
}

impl<T> Mutex<T> {
    /// See `mtx_init` on the PS4 for a reference.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            owning: AtomicUsize::new(MTX_UNOWNED),
            phantom: PhantomData,
        }
    }

    /// See `_mtx_lock_flags` on the PS4 for a reference.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // Check if the current thread can sleep.
        let td = current_thread();

        if !td.can_sleep() {
            panic!("locking a mutex in a non-sleeping context is not supported");
        }

        // Take ownership.
        if self
            .owning
            .compare_exchange(
                MTX_UNOWNED,
                BorrowedArc::as_ptr(&td) as usize,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            todo!()
        }

        td.set_active_mutexes(td.active_mutexes() + 1);

        MutexGuard {
            data: self.data.get(),
            lock: &self.owning,
            phantom: PhantomData,
        }
    }

    /// See `_mtx_unlock_flags` on the PS4 for a reference.
    ///
    /// # Safety
    /// Must be called by the thread that own `lock`.
    unsafe fn unlock(lock: &AtomicUsize) {
        let td = current_thread();

        td.set_active_mutexes(td.active_mutexes() - 1);

        // TODO: There is a check for (m->lock_object).lo_data == 0 on the PS4.
        if lock
            .compare_exchange(
                BorrowedArc::as_ptr(&td) as usize,
                MTX_UNOWNED,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_err()
        {
            todo!()
        }
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

/// An RAII implementation of a "scoped lock" of a mutex. When this structure is dropped (falls out
/// of scope), the lock will be unlocked.
///
/// This struct must not implement [`Send`].
pub struct MutexGuard<'a, T> {
    data: *mut T,
    lock: *const AtomicUsize,
    phantom: PhantomData<&'a Mutex<T>>,
}

impl<'a, T> MutexGuard<'a, T> {
    pub fn map<O, F>(this: Self, f: F) -> MappedMutex<'a, O>
    where
        F: FnOnce(&'a mut T) -> O + 'a,
    {
        let data = unsafe { f(&mut *this.data) };
        let lock = this.lock;

        core::mem::forget(this);

        MappedMutex {
            data,
            lock,
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: This struct does not implement Send.
        unsafe { Mutex::<T>::unlock(&*self.lock) };
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

/// An RAII mutex guard returned by [`MutexGuard::map()`].
///
/// This struct must not implement [`Send`].
pub struct MappedMutex<'a, T> {
    data: T,
    lock: *const AtomicUsize,
    phantom: PhantomData<&'a Mutex<T>>,
}

impl<T> Drop for MappedMutex<'_, T> {
    fn drop(&mut self) {
        // SAFETY: This struct does not implement Send.
        unsafe { Mutex::<T>::unlock(&*self.lock) };
    }
}

impl<T> Deref for MappedMutex<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for MappedMutex<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

unsafe impl<T: Sync> Sync for MappedMutex<'_, T> {}
