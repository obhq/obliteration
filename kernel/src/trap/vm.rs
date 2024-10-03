use super::TrapFrame;
use core::hint::unreachable_unchecked;
use core::ptr::{addr_of_mut, write_volatile};
use obconf::{KernelExit, Vm, VmmMemory};

/// # Interupt safety
/// This function can be called from interupt handler.
pub fn interrupt_handler(env: &Vm, _: &mut TrapFrame) {
    // TODO: Implement a virtual device with GDB stub.
    let vmm = env.vmm as *mut VmmMemory;

    unsafe { write_volatile(addr_of_mut!((*vmm).shutdown), KernelExit::Panic) };
    unsafe { unreachable_unchecked() };
}
