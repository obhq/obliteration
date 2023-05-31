use iced_x86::Register;

/// Contains a CPU states at a certain point in the function.
pub(super) struct CpuState {
    rax: ValueState,
    rbp: ValueState,
    rbx: ValueState,
    rcx: ValueState,
    rdi: ValueState,
    rdx: ValueState,
    rsi: ValueState,
    r8: ValueState,
    r9: ValueState,
    r10: ValueState,
    r11: ValueState,
    r12: ValueState,
    r13: ValueState,
    r14: ValueState,
    r15: ValueState,
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            rax: ValueState::FromCaller,
            rbp: ValueState::FromCaller,
            rbx: ValueState::FromCaller,
            rcx: ValueState::FromCaller,
            rdi: ValueState::FromCaller,
            rdx: ValueState::FromCaller,
            rsi: ValueState::FromCaller,
            r8: ValueState::FromCaller,
            r9: ValueState::FromCaller,
            r10: ValueState::FromCaller,
            r11: ValueState::FromCaller,
            r12: ValueState::FromCaller,
            r13: ValueState::FromCaller,
            r14: ValueState::FromCaller,
            r15: ValueState::FromCaller,
        }
    }

    pub fn register(&self, r: Register) -> &ValueState {
        match r {
            Register::RAX => &self.rax,
            Register::RBP => &self.rbp,
            Register::RBX => &self.rbx,
            Register::RCX => &self.rcx,
            Register::RDI => &self.rdi,
            Register::RDX => &self.rdx,
            Register::RSI => &self.rsi,
            Register::R8 => &self.r8,
            Register::R9 => &self.r9,
            Register::R10 => &self.r10,
            Register::R11 => &self.r11,
            Register::R12 => &self.r12,
            Register::R13 => &self.r13,
            Register::R14 => &self.r14,
            Register::R15 => &self.r15,
            v => panic!("Register {v:?} is not implemented yet."),
        }
    }

    pub fn set_register(&mut self, r: Register, s: ValueState) {
        match r {
            Register::RAX => self.rax = s,
            Register::RBP => self.rbp = s,
            Register::RBX => self.rbx = s,
            Register::RCX => self.rcx = s,
            Register::RDI => self.rdi = s,
            Register::RDX => self.rdx = s,
            Register::RSI => self.rsi = s,
            Register::R8 => self.r8 = s,
            Register::R9 => self.r9 = s,
            Register::R10 => self.r10 = s,
            Register::R11 => self.r11 = s,
            Register::R12 => self.r12 = s,
            Register::R13 => self.r13 = s,
            Register::R14 => self.r14 = s,
            Register::R15 => self.r15 = s,
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
