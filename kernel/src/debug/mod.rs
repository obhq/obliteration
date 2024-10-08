use core::ptr::{addr_of_mut, write_volatile};
use obconf::{DebuggerMemory, StopReason, Vm};

pub fn wait_debugger(env: &Vm) {
    let debug = env.debugger as *mut DebuggerMemory;

    if debug.is_null() {
        return;
    }

    unsafe { write_volatile(addr_of_mut!((*debug).stop), StopReason::WaitForDebugger) };
}
