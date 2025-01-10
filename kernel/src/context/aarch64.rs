use super::Base;
use crate::proc::Thread;
use core::marker::PhantomPinned;
use core::pin::Pin;

/// Contains data passed from CPU setup function for context activation.
pub struct ContextArgs {}

/// Extended [Base] for AArch64.
#[repr(C)]
pub(super) struct Context {
    pub base: Base, // Must be first field.
    phantom: PhantomPinned,
}

impl Context {
    pub fn new(base: Base, args: ContextArgs) -> Self {
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
