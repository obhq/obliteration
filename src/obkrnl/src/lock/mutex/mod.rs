use super::MTX_UNOWNED;
use crate::context::Context;
use alloc::rc::Rc;
use alloc::sync::Arc;
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
    pub fn new(data: T) -> Self {
        // This function is not allowed to access the CPU context due to it can be called before the
        // context has been activated.
        Self {
            data: UnsafeCell::new(data),
            owning: AtomicUsize::new(MTX_UNOWNED),
            phantom: PhantomData,
        }
    }

    /// See `_mtx_lock_flags` on the PS4 for a reference.
    pub fn lock(&self) -> MutexGuard<T> {
        // Disallow locking in an interupt handler.
        let td = Context::thread();

        if td.active_interrupts() != 0 {
            panic!("locking a mutex in an interupt handler is not supported");
        }

        // Take ownership.
        if self
            .owning
            .compare_exchange(
                MTX_UNOWNED,
                Arc::as_ptr(&td) as usize,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            todo!()
        }

        td.active_mutexes().fetch_add(1, Ordering::Relaxed);

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
        let td = Context::thread();

        td.active_mutexes().fetch_sub(1, Ordering::Relaxed);

        // TODO: There is a check for (m->lock_object).lo_data == 0 on the PS4.
        if lock
            .compare_exchange(
                Arc::as_ptr(&td) as usize,
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

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        // SAFETY: This struct does not implement Send.
        unsafe { Mutex::<T>::unlock(&*self.lock) };
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

unsafe impl<'a, T: Sync> Sync for MutexGuard<'a, T> {}

/// An RAII mutex guard returned by [`MutexGuard::map()`].
///
/// This struct must not implement [`Send`].
pub struct MappedMutex<'a, T> {
    data: T,
    lock: *const AtomicUsize,
    phantom: PhantomData<&'a Mutex<T>>,
}

impl<'a, T> Drop for MappedMutex<'a, T> {
    fn drop(&mut self) {
        // SAFETY: This struct does not implement Send.
        unsafe { Mutex::<T>::unlock(&*self.lock) };
    }
}

impl<'a, T> Deref for MappedMutex<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> DerefMut for MappedMutex<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

unsafe impl<'a, T: Sync> Sync for MappedMutex<'a, T> {}
