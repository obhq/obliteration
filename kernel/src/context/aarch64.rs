use crate::proc::Thread;

/// Extended [Context](super::Context) for AArch64.
#[repr(C)]
pub struct Context {
    base: super::Context, // Must be first field.
}

impl Context {
    pub fn new(base: super::Context) -> Self {
        Self { base }
    }
}

pub unsafe fn activate(_: *mut Context) {
    todo!();
}

pub unsafe fn load_fixed_ptr<const O: usize, T>() -> *const T {
    todo!()
}

pub unsafe fn load_usize<const O: usize>() -> usize {
    todo!()
}
