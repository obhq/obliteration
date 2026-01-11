use super::{RegisterAlias, RegisterCategory, RegisterFormat, RegisterType};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::fmt::{Display, Formatter};
use std::mem::offset_of;

/// Register identifier for x86-64.
///
/// Note that we cannot have a gap between the variant since the debugger will treat absent number
/// as the end of register list.
#[repr(usize)]
#[derive(Clone, Copy, IntoPrimitive, TryFromPrimitive)]
pub(super) enum Register {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    Rbp,
    Rsp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Rip,
    Rflags,
}

impl Register {
    pub fn ty(self) -> RegisterType {
        match self {
            Self::Rax => RegisterType::Unsigned,
            Self::Rbx => RegisterType::Unsigned,
            Self::Rcx => RegisterType::Unsigned,
            Self::Rdx => RegisterType::Unsigned,
            Self::Rsi => RegisterType::Unsigned,
            Self::Rdi => RegisterType::Unsigned,
            Self::Rbp => RegisterType::Unsigned,
            Self::Rsp => RegisterType::Unsigned,
            Self::R8 => RegisterType::Unsigned,
            Self::R9 => RegisterType::Unsigned,
            Self::R10 => RegisterType::Unsigned,
            Self::R11 => RegisterType::Unsigned,
            Self::R12 => RegisterType::Unsigned,
            Self::R13 => RegisterType::Unsigned,
            Self::R14 => RegisterType::Unsigned,
            Self::R15 => RegisterType::Unsigned,
            Self::Rip => RegisterType::Unsigned,
            Self::Rflags => RegisterType::Unsigned,
        }
    }

    pub fn category(self) -> RegisterCategory {
        match self {
            Self::Rax => RegisterCategory::General,
            Self::Rbx => RegisterCategory::General,
            Self::Rcx => RegisterCategory::General,
            Self::Rdx => RegisterCategory::General,
            Self::Rsi => RegisterCategory::General,
            Self::Rdi => RegisterCategory::General,
            Self::Rbp => RegisterCategory::General,
            Self::Rsp => RegisterCategory::General,
            Self::R8 => RegisterCategory::General,
            Self::R9 => RegisterCategory::General,
            Self::R10 => RegisterCategory::General,
            Self::R11 => RegisterCategory::General,
            Self::R12 => RegisterCategory::General,
            Self::R13 => RegisterCategory::General,
            Self::R14 => RegisterCategory::General,
            Self::R15 => RegisterCategory::General,
            Self::Rip => RegisterCategory::General,
            Self::Rflags => RegisterCategory::General,
        }
    }

    pub fn alias(self) -> Option<RegisterAlias> {
        match self {
            Self::Rbp => Some(RegisterAlias::FramePointer),
            Self::Rsp => Some(RegisterAlias::StackPointer),
            Self::Rip => Some(RegisterAlias::ProgramCounter),
            _ => None,
        }
    }

    /// Returns size of the register, in bits.
    pub fn size(self) -> u8 {
        match self {
            Self::Rax => 64,
            Self::Rbx => 64,
            Self::Rcx => 64,
            Self::Rdx => 64,
            Self::Rsi => 64,
            Self::Rdi => 64,
            Self::Rbp => 64,
            Self::Rsp => 64,
            Self::R8 => 64,
            Self::R9 => 64,
            Self::R10 => 64,
            Self::R11 => 64,
            Self::R12 => 64,
            Self::R13 => 64,
            Self::R14 => 64,
            Self::R15 => 64,
            Self::Rip => 64,
            Self::Rflags => 64,
        }
    }

    /// Returns offset of the register within `g` and `G` response.
    pub fn offset(self) -> usize {
        match self {
            Self::Rax => offset_of!(Registers, rax),
            Self::Rbx => offset_of!(Registers, rbx),
            Self::Rcx => offset_of!(Registers, rcx),
            Self::Rdx => offset_of!(Registers, rdx),
            Self::Rsi => offset_of!(Registers, rsi),
            Self::Rdi => offset_of!(Registers, rdi),
            Self::Rbp => offset_of!(Registers, rbp),
            Self::Rsp => offset_of!(Registers, rsp),
            Self::R8 => offset_of!(Registers, r8),
            Self::R9 => offset_of!(Registers, r9),
            Self::R10 => offset_of!(Registers, r10),
            Self::R11 => offset_of!(Registers, r11),
            Self::R12 => offset_of!(Registers, r12),
            Self::R13 => offset_of!(Registers, r13),
            Self::R14 => offset_of!(Registers, r14),
            Self::R15 => offset_of!(Registers, r15),
            Self::Rip => offset_of!(Registers, rip),
            Self::Rflags => offset_of!(Registers, rflags),
        }
    }

    pub fn format(self) -> RegisterFormat {
        match self {
            Self::Rax => RegisterFormat::Hex,
            Self::Rbx => RegisterFormat::Hex,
            Self::Rcx => RegisterFormat::Hex,
            Self::Rdx => RegisterFormat::Hex,
            Self::Rsi => RegisterFormat::Hex,
            Self::Rdi => RegisterFormat::Hex,
            Self::Rbp => RegisterFormat::Hex,
            Self::Rsp => RegisterFormat::Hex,
            Self::R8 => RegisterFormat::Hex,
            Self::R9 => RegisterFormat::Hex,
            Self::R10 => RegisterFormat::Hex,
            Self::R11 => RegisterFormat::Hex,
            Self::R12 => RegisterFormat::Hex,
            Self::R13 => RegisterFormat::Hex,
            Self::R14 => RegisterFormat::Hex,
            Self::R15 => RegisterFormat::Hex,
            Self::Rip => RegisterFormat::Hex,
            Self::Rflags => RegisterFormat::Hex,
        }
    }

    /// Returns DWARF register number according to https://gitlab.com/x86-psABIs/x86-64-ABI (Figure
    /// 3.36: DWARF Register Number Mapping).
    pub fn dwarf_number(self) -> usize {
        match self {
            Self::Rax => 0,
            Self::Rbx => 3,
            Self::Rcx => 2,
            Self::Rdx => 1,
            Self::Rsi => 4,
            Self::Rdi => 5,
            Self::Rbp => 6,
            Self::Rsp => 7,
            Self::R8 => 8,
            Self::R9 => 9,
            Self::R10 => 10,
            Self::R11 => 11,
            Self::R12 => 12,
            Self::R13 => 13,
            Self::R14 => 14,
            Self::R15 => 15,
            Self::Rip => 16,
            Self::Rflags => 49,
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Rax => "rax",
            Self::Rbx => "rbx",
            Self::Rcx => "rcx",
            Self::Rdx => "rdx",
            Self::Rsi => "rsi",
            Self::Rdi => "rdi",
            Self::Rbp => "rbp",
            Self::Rsp => "rsp",
            Self::R8 => "r8",
            Self::R9 => "r9",
            Self::R10 => "r10",
            Self::R11 => "r11",
            Self::R12 => "r12",
            Self::R13 => "r13",
            Self::R14 => "r14",
            Self::R15 => "r15",
            Self::Rip => "rip",
            Self::Rflags => "rflags",
        })
    }
}

/// Raw content for `g` and `G` response.
#[repr(C, packed)]
pub struct Registers {
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rbp: usize,
    pub rsp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub rip: usize,
    pub rflags: usize,
}
