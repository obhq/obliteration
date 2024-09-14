use super::Context;
use crate::proc::Thread;

pub unsafe fn activate(_: *mut Context) {
    todo!();
}

pub unsafe fn thread() -> *const Thread {
    todo!();
}

pub unsafe fn cpu() -> usize {
    todo!();
}
