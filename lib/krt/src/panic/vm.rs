use crate::phys_vaddr;
use config::{KernelExit, Vm, VmmMemory};
use core::hint::unreachable_unchecked;
use core::ptr::{addr_of_mut, write_volatile};

pub fn panic(env: &Vm) -> ! {
    let vmm = (phys_vaddr() + env.vmm) as *mut VmmMemory;

    unsafe { write_volatile(addr_of_mut!((*vmm).shutdown), KernelExit::Panic) };
    unsafe { unreachable_unchecked() };
}
