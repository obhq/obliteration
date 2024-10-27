use super::Base;
use crate::proc::Thread;

/// Contains data passed from CPU setup function for context activation.
pub struct ContextArgs {}

/// Extended [Base] for AArch64.
#[repr(C)]
pub(super) struct Context {
    base: Base, // Must be first field.
}

impl Context {
    pub fn new(base: Base, args: ContextArgs) -> Self {
        Self { base }
    }

    pub unsafe fn activate(&mut self) {
        todo!();
    }

    pub unsafe fn load_fixed_ptr<const O: usize, T>() -> *const T {
        todo!()
    }

    pub unsafe fn load_usize<const O: usize>() -> usize {
        todo!()
    }
}
