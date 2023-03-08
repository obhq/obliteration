use iced_x86::Register;

/// Contains a CPU states at a certain point in the function.
pub(super) struct CpuState {
    rbp: ValueState,
    rdi: ValueState,
    r12: ValueState,
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            rbp: ValueState::FromCaller,
            rdi: ValueState::FromCaller,
            r12: ValueState::FromCaller,
        }
    }

    pub fn register(&self, r: Register) -> &ValueState {
        match r {
            Register::RBP => &self.rbp,
            Register::RDI => &self.rdi,
            Register::R12 => &self.r12,
            v => panic!("Register {v:?} is not implemented yet."),
        }
    }

    pub fn set_register(&mut self, r: Register, s: ValueState) {
        match r {
            Register::RBP => self.rbp = s,
            Register::RDI => self.rdi = s,
            Register::R12 => self.r12 = s,
            v => panic!("Register {v:?} is not implemented yet."),
        }
    }
}

/// Represents a state for each value in the CPU.
pub(super) enum ValueState {
    FromCaller,
    Param(usize),
    Local,
}
