use crate::context::Context;
use crate::proc::Thread;
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
    const OWNER_NONE: usize = 4; // MTX_UNOWNED

    /// See `mtx_init` on the PS4 for a reference.
    pub fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            owning: AtomicUsize::new(Self::OWNER_NONE),
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
                Self::OWNER_NONE,
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
            mtx: self,
            td,
            phantom: PhantomData,
        }
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

/// An RAII implementation of a "scoped lock" of a mutex. When this structure is dropped (falls out
/// of scope), the lock will be unlocked.
pub struct MutexGuard<'a, T> {
    mtx: &'a Mutex<T>,
    td: Arc<Thread>,
    phantom: PhantomData<Rc<()>>, // For !Send and !Sync.
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        // This method basically an implementation of _mtx_unlock_flags.
        self.td.active_mutexes().fetch_sub(1, Ordering::Relaxed);

        // TODO: There is a check for (m->lock_object).lo_data == 0 on the PS4.
        if self
            .mtx
            .owning
            .compare_exchange(
                Arc::as_ptr(&self.td) as usize,
                Mutex::<T>::OWNER_NONE,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_err()
        {
            todo!()
        }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mtx.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mtx.data.get() }
    }
}

unsafe impl<'a, T: Sync> Sync for MutexGuard<'a, T> {}
