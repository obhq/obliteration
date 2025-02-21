use super::Thread;
use crate::context::{BorrowedArc, current_thread};
use core::cell::Cell;

/// Encapsulates a field of [Thread] that can only be accessed by the CPU that currently executing
/// the thread.
///
/// # Context safety
/// [`Default`] implementation of this type does not require a CPU context as long as implementation
/// on `T` does not.
#[derive(Default)]
pub struct PrivateCell<T>(T);

impl<T> PrivateCell<T> {
    fn validate(&self, owner: &Thread) {
        // This check will optimized out for most of the time due to the implementation of
        // current_thread() use "pure" + "nomem" on inline assembly.
        let current = current_thread();

        if !core::ptr::eq(BorrowedArc::as_ptr(&current), owner) {
            panic!("accessing a private cell from the other thread is not supported");
        }
    }
}

impl<T> PrivateCell<Cell<T>> {
    /// See [set] for a safe wrapper.
    ///
    /// # Safety
    /// `owner` must be an owner of this field.
    ///
    /// # Panics
    /// If `owner` is not the current thread.
    pub unsafe fn set(&self, owner: &Thread, v: T) {
        self.validate(owner);
        self.0.set(v);
    }
}

impl<T: Copy> PrivateCell<Cell<T>> {
    /// See [get] for a safe wrapper.
    ///
    /// # Safety
    /// `owner` must be an owner of this field.
    ///
    /// # Panics
    /// If `owner` is not the current thread.
    pub unsafe fn get(&self, owner: &Thread) -> T {
        self.validate(owner);
        self.0.get()
    }
}

unsafe impl<T: Send> Sync for PrivateCell<T> {}

/// Safe wrapper of [PrivateCell::set()].
macro_rules! set {
    ($t:ident, $f:ident, $v:expr) => {
        // SAFETY: $t is an owner of $f.
        unsafe { $t.$f.set($t, $v) }
    };
}

/// Safe wrapper of [PrivateCell::get()].
macro_rules! get {
    ($t:ident, $f:ident) => {
        // SAFETY: $t is an owner of $f.
        unsafe { $t.$f.get($t) }
    };
}

pub(super) use get;
pub(super) use set;
