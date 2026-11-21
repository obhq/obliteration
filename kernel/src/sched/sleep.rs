use crate::context::current_thread;

/// See `_sleep` on the PS4 for a reference.
pub fn sleep() {
    let td = current_thread();

    if !td.can_sleep() {
        panic!("attempt to sleep in non-sleeping context");
    }

    // SAFETY: td is the execution thread and td.can_sleep() already check if called from interrupt
    // handler.
    let addr = unsafe { td.sleeping().get() };

    // Remove current thread from sleep queue.
    if addr != 0 {
        todo!()
    }

    todo!()
}
