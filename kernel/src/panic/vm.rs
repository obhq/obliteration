use core::hint::unreachable_unchecked;
use core::ptr::{addr_of_mut, write_volatile};
use obconf::{KernelExit, Vm, VmmMemory};

/// # Context safety
/// This function does not require a CPU context.
///
/// # Interupt safety
/// This function is interupt safe.
pub fn panic(env: &Vm) -> ! {
    let vmm = env.vmm as *mut VmmMemory;

    unsafe { write_volatile(addr_of_mut!((*vmm).shutdown), KernelExit::Panic) };
    unsafe { unreachable_unchecked() };
}
