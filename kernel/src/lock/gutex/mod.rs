pub use self::guard::*;

use super::MTX_UNOWNED;
use crate::context::{current_thread, BorrowedArc};
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicUsize, Ordering};

mod guard;

/// A mutex that grant exclusive access to a group of members.
///
/// The [`crate::lock::Mutex`] is prone to deadlock when using on a multiple struct fields like
/// this:
///
/// ```
/// use crate::lock::Mutex;
///
/// pub struct Foo {
///     field1: Mutex<()>,
///     field2: Mutex<()>,
/// }
/// ```
///
/// The order to acquire the lock must be the same everywhere otherwise the deadlock is possible.
/// Maintaining the lock order manually are cumbersome task so we introduce this type to handle this
/// instead.
///
/// How this type are working is simple. Any locks on any member will lock the same mutex in the
/// group, which mean there are only one mutex in the group. It have the same effect as the
/// following code:
///
/// ```
/// use crate::lock::Mutex;
///
/// pub struct Foo {
///     data: Mutex<Data>,
/// }
///
/// struct Data {
///     field1: (),
///     field2: (),
/// }
/// ```
///
/// The bonus point of this type is it will allow recursive lock for read-only access so you will
/// never end up deadlock yourself. It will panic if you try to acquire write access while the
/// readers are still active the same as [`core::cell::RefCell`].
pub struct Gutex<T> {
    group: Arc<GutexGroup>,
    active: UnsafeCell<usize>,
    value: UnsafeCell<T>,
}

impl<T> Gutex<T> {
    /// Locks this [`Gutex`] with read-only access.
    ///
    /// Multiple read-only accesses can be taken out at the same time.
    ///
    /// # Panics
    /// If there are an active write access to this [`Gutex`].
    pub fn read(&self) -> GutexRead<T> {
        // Check if there are an active writer.
        let lock = self.group.lock();
        let active = self.active.get();

        // SAFETY: This is safe because we own the lock that protect both active and value.
        unsafe {
            if *active == usize::MAX {
                panic!("attempt to acquire the read lock while there are an active write lock");
            } else if *active == (usize::MAX - 1) {
                // This should never happen because stack overflow should be triggering first.
                panic!("maximum number of active readers has been reached");
            }

            *active += 1;

            GutexRead::new(lock, active, self.value.get())
        }
    }

    /// # Panics
    /// If there are any active reader or writer.
    pub fn write(&self) -> GutexWrite<T> {
        // Check if there are active reader or writer.
        let lock = self.group.lock();
        let active = self.active.get();

        // SAFETY: This is safe because we own the lock that protect both active and value.
        unsafe {
            if *active != 0 {
                panic!(
                    "attempt to acquire the write lock while there are an active reader or writer"
                );
            }

            *active = usize::MAX;

            GutexWrite::new(lock, active, self.value.get())
        }
    }
}

unsafe impl<T: Send> Send for Gutex<T> {}
unsafe impl<T: Send> Sync for Gutex<T> {}

/// Group of [`Gutex`].
pub struct GutexGroup {
    owning: AtomicUsize,
    active: UnsafeCell<usize>,
}

impl GutexGroup {
    /// # Context safety
    /// This function does not require a CPU context on **stage 1** heap.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            owning: AtomicUsize::new(MTX_UNOWNED),
            active: UnsafeCell::new(0),
        })
    }

    /// # Context safety
    /// This function does not require a CPU context if [`Default`] implementation on `T` does not.
    pub fn spawn_default<T: Default>(self: Arc<Self>) -> Gutex<T> {
        self.spawn(T::default())
    }

    /// # Context safety
    /// This function does not require a CPU context.
    pub fn spawn<T>(self: Arc<Self>, value: T) -> Gutex<T> {
        Gutex {
            group: self,
            active: UnsafeCell::new(0),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> GroupGuard {
        // Acquire the lock.
        let td = current_thread();
        let id = BorrowedArc::as_ptr(&td) as usize;

        loop {
            let owning = match self.owning.compare_exchange(
                MTX_UNOWNED,
                id,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => v,
            };

            if owning == id {
                break;
            }

            self.wait();
        }

        // SAFETY: This is safe because the current thread acquire the lock successfully by the
        // above compare_exchange().
        unsafe { GroupGuard::new(self) }
    }

    #[inline(never)]
    fn wait(&self) {
        todo!()
    }
}

unsafe impl Send for GutexGroup {}
unsafe impl Sync for GutexGroup {}

/// An RAII object used to release the lock on [`GutexGroup`]. This type cannot be send because it
/// will cause data race on the group when dropping if more than one [`GroupGuard`] are active.
struct GroupGuard<'a> {
    group: &'a GutexGroup,
    phantom: PhantomData<Rc<()>>, // For !Send and !Sync.
}

impl<'a> GroupGuard<'a> {
    /// # Safety
    /// The group must be locked by the calling thread with no active references to any of its
    /// field.
    unsafe fn new(group: &'a GutexGroup) -> Self {
        *group.active.get() += 1;

        Self {
            group,
            phantom: PhantomData,
        }
    }

    #[inline(never)]
    fn release(&mut self) {
        self.group.owning.store(MTX_UNOWNED, Ordering::Release);

        todo!("wakeup waiting thread");
    }
}

impl Drop for GroupGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            let active = self.group.active.get();

            *active -= 1;

            if *active != 0 {
                return;
            }
        }

        self.release();
    }
}
