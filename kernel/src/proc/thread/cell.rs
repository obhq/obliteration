use super::Thread;
use crate::context::{current_thread, BorrowedArc};
use core::cell::{RefCell, RefMut};

/// Encapsulates a field of [Thread] that can only be accessed by the CPU that currently executing
/// the thread.
pub struct PrivateCell<T>(RefCell<T>);

impl<T> PrivateCell<T> {
    pub fn new(v: T) -> Self {
        Self(RefCell::new(v))
    }

    /// # Safety
    /// `owner` must be an owner of this field.
    ///
    /// # Panics
    /// If `owner` is not the current thread.
    pub unsafe fn borrow_mut(&self, owner: &Thread) -> RefMut<T> {
        self.validate(owner);
        self.0.borrow_mut()
    }

    fn validate(&self, owner: &Thread) {
        let current = current_thread();

        if !core::ptr::eq(BorrowedArc::as_ptr(&current), owner) {
            panic!("accessing a private cell from the other thread is not supported");
        }
    }
}

unsafe impl<T> Sync for PrivateCell<T> {}
