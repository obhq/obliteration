use core::cell::Cell;

/// Encapsulates a field of [Thread](super::Thread) that can only be accessed by the CPU that
/// currently executing the thread.
pub struct LocalCell<T>(T);

impl<T: Send> LocalCell<T> {
    /// # Context safety
    /// This function does not require a CPU context.
    pub(super) fn new(v: T) -> Self {
        Self(v)
    }
}

impl<T> LocalCell<T> {
    /// # Safety
    /// This method cannot be called by the other thread or interrupt handler.
    pub unsafe fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: Copy> LocalCell<Cell<T>> {
    /// # Safety
    /// This method cannot be called by the other thread or interrupt handler.
    pub unsafe fn get(&self) -> T {
        self.0.get()
    }
}

unsafe impl<T> Send for LocalCell<T> {}
unsafe impl<T> Sync for LocalCell<T> {}
