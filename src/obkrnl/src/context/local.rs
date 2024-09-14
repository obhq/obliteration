use super::{Context, PinnedContext};
use crate::config::config;
use alloc::vec::Vec;
use core::ops::Deref;

/// Encapsulates per-CPU value.
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

    pub fn lock(&self) -> CpuLock<T> {
        let pin = Context::pin();
        let val = &self.0[unsafe { pin.cpu() }];

        CpuLock { val, pin }
    }
}

/// RAII struct to access per-CPU value in [`CpuLocal`].
pub struct CpuLock<'a, T> {
    val: &'a T,
    #[allow(dead_code)]
    pin: PinnedContext, // Must be dropped last.
}

impl<'a, T> Deref for CpuLock<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.val
    }
}
