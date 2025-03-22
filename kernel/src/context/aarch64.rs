use super::Base;
use crate::arch::ArchConfig;
use crate::proc::Thread;
use core::marker::PhantomPinned;
use core::pin::Pin;

/// Extended [Base] for AArch64.
#[repr(C)]
pub(super) struct Context {
    pub base: Base, // Must be first field.
    phantom: PhantomPinned,
}

impl Context {
    pub fn new(base: Base, arch: &ArchConfig) -> Self {
        Self {
            base,
            phantom: PhantomPinned,
        }
    }

    pub unsafe fn activate(self: Pin<&mut Self>) {
        todo!();
    }

    pub unsafe fn load_static_ptr<const O: usize, T>() -> *const T {
        todo!()
    }

    pub unsafe fn load_ptr<const O: usize, T>() -> *const T {
        todo!()
    }

    pub unsafe fn load_volatile_usize<const O: usize>() -> usize {
        todo!()
    }
}
