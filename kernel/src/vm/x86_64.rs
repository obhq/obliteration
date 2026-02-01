use super::VmPage;
use crate::config::PAGE_SIZE;
use core::arch::asm;
use krt::phys_vaddr;

impl VmPage {
    /// See `pagezero` on the Orbis for a reference.
    ///
    /// # Safety
    /// The caller must have exclusive access to this page and no any references to the data within
    /// this page.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x2DDD70|
    pub unsafe fn fill_with_zeros(&self) {
        // The Orbis also check if the address within the stack but I don't think we need that.
        let addr = phys_vaddr() + self.addr;

        unsafe {
            asm!(
            "sub {addr}, {i}",
            "xor eax, eax",
            "2:",
            "movnti [{addr} + {i}], rax",
            "movnti [{addr} + {i} + 0x08], rax",
            "movnti [{addr} + {i} + 0x10], rax",
            "movnti [{addr} + {i} + 0x18], rax",
            "add {i}, 0x20",
            "jnz 2b",
            "sfence",
            addr = inout(reg) addr => _,
            i = inout(reg) { -(PAGE_SIZE.get() as isize) } => _,
            out("rax") _)
        };
    }
}
