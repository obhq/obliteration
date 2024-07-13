use bitflags::bitflags;

/// Implementation of `pcb` structure.
#[derive(Default)]
pub struct Pcb {
    pub r15: usize,      // pcb_r15
    pub r14: usize,      // pcb_r14
    pub r13: usize,      // pcb_r13
    pub r12: usize,      // pcb_r12
    pub rbp: usize,      // pcb_rbp
    pub rsp: usize,      // pcb_rsp
    pub rbx: usize,      // pcb_rbx
    pub rip: usize,      // pcb_rip
    pub fsbase: usize,   // pcb_fsbase
    pub flags: PcbFlags, // pcb_flags
}

bitflags! {
    /// Flags of [`Pcb`].
    #[derive(Default)]
    pub struct PcbFlags: u32 {
        const PCB_FULL_IRET = 0x01;
    }
}
