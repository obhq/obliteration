use iced_x86::Register;

/// Contains a CPU states at a certain point in the function.
pub(super) struct CpuState {
    rbp: ValueState,
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            rbp: ValueState::FromCaller,
        }
    }

    pub fn set_register(&mut self, r: Register, s: ValueState) {
        match r {
            Register::RBP => self.rbp = s,
            v => panic!("Register {v:?} is not implemented yet."),
        }
    }
}

/// Represents a state for each value in the CPU.
pub(super) enum ValueState {
    FromCaller,
    Zero,
}
