use super::{pin_cpu, PinnedContext};
use crate::config::config;
use alloc::vec::Vec;
use core::ops::Deref;

/// Encapsulates per-CPU value.
///
/// `T` need to implement [Send] for this type to implement [Send] and [Sync] because its value
/// created by one thread then access from another thread.
///
/// Use [RefCell](core::cell::RefCell) if you need interior mutability but it will make that value
/// not safe to access from any interrupt handler. You can't use mutex here because once the thread
/// is pinned it cannot go to sleep.
pub struct CpuLocal<T>(Vec<T>);

impl<T> CpuLocal<T> {
    pub fn new(mut f: impl FnMut(usize) -> T) -> Self {
        let len = config().max_cpu.get();
        let mut vec = Vec::with_capacity(len);

        for i in 0..len {
            vec.push(f(i));
        }

        Self(vec)
    }

    /// The calling thread cannot go to sleep until the returned [`CpuLock`] is dropped. Attempt to
    /// call any function that can put the thread to sleep will be panic.
    pub fn lock(&self) -> CpuLock<T> {
        let pin = pin_cpu();
        let val = &self.0[unsafe { pin.cpu() }];

        CpuLock { val, pin }
    }
}

unsafe impl<T: Send> Send for CpuLocal<T> {}
unsafe impl<T: Send> Sync for CpuLocal<T> {}

/// RAII struct to access per-CPU value in [`CpuLocal`].
pub struct CpuLock<'a, T> {
    val: &'a T,
    #[allow(dead_code)]
    pin: PinnedContext, // Must be dropped last.
}

impl<T> Deref for CpuLock<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.val
    }
}
