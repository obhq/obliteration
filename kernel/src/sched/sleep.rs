use crate::context::current_thread;

/// See `_sleep` on the PS4 for a reference.
pub fn sleep() {
    // Remove current thread from sleep queue.
    let td = current_thread();
    let addr = td.sleeping_mut();

    if *addr != 0 {
        todo!()
    }

    todo!()
}
