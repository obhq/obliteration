pub use self::guard::*;
use std::cell::UnsafeCell;
use std::io::Error;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

mod guard;

/// A mutex that grant exclusive access to a group of members.
///
/// The [`std::sync::Mutex`] and the related types are prone to deadlock when using on a multiple
/// struct fields like this:
///
/// ```
/// use std::sync::Mutex;
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
/// use std::sync::Mutex;
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
/// readers are still active the same as [`std::cell::RefCell`].
#[derive(Debug)]
pub struct Gutex<T> {
    group: Arc<GutexGroup>,
    active: UnsafeCell<usize>,
    value: UnsafeCell<T>,
}

impl<T> Gutex<T> {
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// # Panics
    /// If there are an active writer.
    pub fn read(&self) -> GutexReadGuard<'_, T> {
        // Check if there are an active writer.
        let lock = self.group.lock();
        let active = self.active.get();

        unsafe {
            if *active == usize::MAX {
                panic!("attempt to acquire the read lock while there are an active write lock");
            } else if *active == (usize::MAX - 1) {
                // This should never happen because stack overflow should be triggering first.
                panic!("maximum number of active readers has been reached");
            }

            *active += 1;
        }

        GutexReadGuard::new(lock, self)
    }

    /// # Panics
    /// If there are any active reader or writer.
    pub fn write(&self) -> GutexWriteGuard<'_, T> {
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

            GutexWriteGuard::new(&mut *active, &mut *self.value.get(), lock)
        }
    }
}

unsafe impl<T: Send> Send for Gutex<T> {}
unsafe impl<T: Send> Sync for Gutex<T> {}

/// Represents a group of [`Gutex`].
#[derive(Debug)]
pub struct GutexGroup {
    owning: ThreadId,
    active: UnsafeCell<usize>,
}

impl GutexGroup {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            owning: ThreadId::new(0),
            active: UnsafeCell::new(0),
        })
    }

    pub fn spawn<T>(self: &Arc<Self>, value: T) -> Gutex<T> {
        Gutex {
            group: self.clone(),
            active: UnsafeCell::new(0),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> GroupGuard<'_> {
        // Check if the calling thread already own the lock.
        let current = Self::current_thread();

        if current == self.owning.load(Ordering::Relaxed) {
            // SAFETY: This is safe because the current thread own the lock.
            return unsafe { GroupGuard::new(self) };
        }

        // Acquire the lock.
        while let Err(owning) =
            self.owning
                .compare_exchange(0, current, Ordering::Acquire, Ordering::Relaxed)
        {
            // Wait for the lock to unlock.
            unsafe { Self::wait_unlock(self.owning.as_ptr(), owning) };
        }

        // SAFETY: This is safe because the current thread acquire the lock successfully by the
        // above compare_exchange().
        unsafe { GroupGuard::new(self) }
    }

    #[cfg(target_os = "linux")]
    fn current_thread() -> i32 {
        unsafe { libc::gettid() }
    }

    #[cfg(target_os = "macos")]
    fn current_thread() -> u64 {
        let mut id = 0;
        assert_eq!(unsafe { libc::pthread_threadid_np(0, &mut id) }, 0);
        id
    }

    #[cfg(target_os = "windows")]
    fn current_thread() -> u32 {
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
    }

    #[cfg(target_os = "linux")]
    unsafe fn wait_unlock(addr: *mut i32, owning: i32) {
        use libc::{syscall, SYS_futex, EAGAIN, FUTEX_PRIVATE_FLAG, FUTEX_WAIT};

        if unsafe { syscall(SYS_futex, addr, FUTEX_WAIT | FUTEX_PRIVATE_FLAG, owning, 0) } < 0 {
            let e = Error::last_os_error();

            if e.raw_os_error().unwrap() != EAGAIN {
                panic!("FUTEX_WAIT failed: {e}");
            }
        }
    }

    #[cfg(target_os = "macos")]
    unsafe fn wait_unlock(addr: *mut u64, owning: u64) {
        use ulock_sys::__ulock_wait;
        use ulock_sys::darwin19::UL_COMPARE_AND_WAIT64;

        if __ulock_wait(UL_COMPARE_AND_WAIT64, addr.cast(), owning, 0) != 0 {
            panic!("__ulock_wait() failed: {}", Error::last_os_error());
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn wait_unlock(addr: *mut u32, owning: u32) {
        use windows_sys::Win32::System::Threading::{WaitOnAddress, INFINITE};

        if unsafe { WaitOnAddress(addr.cast(), &owning as *const u32 as _, 4, INFINITE) } == 0 {
            panic!("WaitOnAddress() failed: {}", Error::last_os_error());
        }
    }

    #[cfg(target_os = "linux")]
    unsafe fn wake_one(addr: *mut i32) {
        use libc::{syscall, SYS_futex, FUTEX_PRIVATE_FLAG, FUTEX_WAKE};

        if unsafe { syscall(SYS_futex, addr, FUTEX_WAKE | FUTEX_PRIVATE_FLAG, 1) } < 0 {
            panic!("FUTEX_WAKE failed: {}", Error::last_os_error());
        }
    }

    #[cfg(target_os = "macos")]
    unsafe fn wake_one(addr: *mut u64) {
        use libc::ENOENT;
        use ulock_sys::__ulock_wake;
        use ulock_sys::darwin19::UL_COMPARE_AND_WAIT64;

        if __ulock_wake(UL_COMPARE_AND_WAIT64, addr.cast(), 0) != 0 {
            // __ulock_wake will return ENOENT if no other threads being waiting on the address.
            let e = Error::last_os_error();

            if e.raw_os_error().unwrap() != ENOENT {
                panic!("__ulock_wake() failed: {e}");
            }
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn wake_one(addr: *mut u32) {
        use windows_sys::Win32::System::Threading::WakeByAddressSingle;

        unsafe { WakeByAddressSingle(addr.cast()) };
    }
}

unsafe impl Send for GutexGroup {}
unsafe impl Sync for GutexGroup {}

/// An RAII object used to release the lock on [`GutexGroup`]. This type cannot be send because it
/// will cause data race on the group when dropping if more than one [`GroupGuard`] are active.
struct GroupGuard<'a> {
    pub group: &'a GutexGroup,
    pub phantom: PhantomData<Rc<i32>>, // For !Send and !Sync.
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
}

impl<'a> Drop for GroupGuard<'a> {
    fn drop(&mut self) {
        // Decrease the active lock.
        unsafe {
            let active = self.group.active.get();

            *active -= 1;

            if *active != 0 {
                return;
            }
        }

        // Release the lock.
        self.group.owning.store(0, Ordering::Release);

        unsafe { GutexGroup::wake_one(self.group.owning.as_ptr()) };
    }
}

#[cfg(target_os = "linux")]
type ThreadId = std::sync::atomic::AtomicI32;

#[cfg(target_os = "macos")]
type ThreadId = std::sync::atomic::AtomicU64;

#[cfg(target_os = "windows")]
type ThreadId = std::sync::atomic::AtomicU32;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::time::Duration;

    #[test]
    fn group_lock() {
        let b = Arc::new(Barrier::new(2));
        let v = Arc::new(GutexGroup::new().spawn(0));
        let mut l = v.write();
        let t = std::thread::spawn({
            let b = b.clone();
            let v = v.clone();

            move || {
                // Wait for parent thread.
                let mut l = v.write();

                b.wait();

                assert_eq!(*l, 1);

                // Notify the parent thread.
                std::thread::sleep(Duration::from_secs(1));

                *l = 2;
            }
        });

        // Notify the inner thread.
        *l = 1;
        drop(l);

        // Wait for the inner thread value.
        b.wait();

        assert_eq!(*v.read(), 2);

        t.join().unwrap();
    }
}
