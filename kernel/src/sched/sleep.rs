use crate::context::Context;

/// See `_sleep` on the PS4 for a reference.
pub fn sleep() {
    // Remove current thread from sleep queue.
    let td = Context::thread();
    let addr = td.sleeping_mut();

    if *addr != 0 {
        todo!()
    }

    todo!()
}
