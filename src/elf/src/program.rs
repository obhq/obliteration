use bitflags::bitflags;
use std::fmt::{Display, Formatter};

/// Contains information for each ELF program.
#[derive(Debug)]
pub struct Program {
    ty: ProgramType,
    flags: ProgramFlags,
    offset: u64,
    addr: usize,
    file_size: u64,
    memory_size: usize,
    alignment: usize,
}

impl Program {
    pub(super) fn new(
        ty: ProgramType,
        flags: ProgramFlags,
        offset: u64,
        addr: usize,
        file_size: u64,
        memory_size: usize,
        alignment: usize,
    ) -> Self {
        Self {
            ty,
            flags,
            offset,
            addr,
            file_size,
            memory_size,
            alignment,
        }
    }

    pub fn ty(&self) -> ProgramType {
        self.ty
    }

    pub fn flags(&self) -> ProgramFlags {
        self.flags
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn addr(&self) -> usize {
        self.addr
    }

    pub fn end(&self) -> usize {
        self.addr + self.memory_size
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn memory_size(&self) -> usize {
        self.memory_size
    }

    pub fn alignment(&self) -> usize {
        self.alignment
    }

    pub fn aligned_size(&self) -> usize {
        Self::align_page(self.memory_size as u64) as usize
    }

    pub(super) fn align_page(v: u64) -> u64 {
        (v + 0x3fff) & 0xffffffffffffc000
    }

    pub(super) fn align_2mb(v: u64) -> u64 {
        (v + 0x1fffff) & 0xffffffffffe00000
    }
}

/// Represents type of an ELF program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramType(u32);

impl ProgramType {
    pub const PT_LOAD: ProgramType = ProgramType(0x00000001);
    pub const PT_DYNAMIC: ProgramType = ProgramType(0x00000002);
    pub const PT_INTERP: ProgramType = ProgramType(0x00000003);
    pub const PT_TLS: ProgramType = ProgramType(0x00000007);
    pub const PT_SCE_DYNLIBDATA: ProgramType = ProgramType(0x61000000);
    pub const PT_SCE_PROCPARAM: ProgramType = ProgramType(0x61000001);
    pub const PT_SCE_MODULE_PARAM: ProgramType = ProgramType(0x61000002);
    pub const PT_SCE_RELRO: ProgramType = ProgramType(0x61000010);
    pub const PT_SCE_COMMENT: ProgramType = ProgramType(0x6fffff00);
    pub const PT_SCE_VERSION: ProgramType = ProgramType(0x6fffff01);
    pub const PT_GNU_EH_FRAME: ProgramType = ProgramType(0x6474e550);

    pub fn new(v: u32) -> Self {
        Self(v)
    }
}

impl Display for ProgramType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::PT_LOAD => f.write_str("PT_LOAD"),
            Self::PT_DYNAMIC => f.write_str("PT_DYNAMIC"),
            Self::PT_INTERP => f.write_str("PT_INTERP"),
            Self::PT_TLS => f.write_str("PT_TLS"),
            Self::PT_SCE_DYNLIBDATA => f.write_str("PT_SCE_DYNLIBDATA"),
            Self::PT_SCE_PROCPARAM => f.write_str("PT_SCE_PROCPARAM"),
            Self::PT_SCE_MODULE_PARAM => f.write_str("PT_SCE_MODULE_PARAM"),
            Self::PT_SCE_RELRO => f.write_str("PT_SCE_RELRO"),
            Self::PT_SCE_COMMENT => f.write_str("PT_SCE_COMMENT"),
            Self::PT_SCE_VERSION => f.write_str("PT_SCE_VERSION"),
            Self::PT_GNU_EH_FRAME => f.write_str("PT_GNU_EH_FRAME"),
            t => write!(f, "{:#010x}", t.0),
        }
    }
}

bitflags! {
    /// Represents flags for an ELF program.
    #[derive(Debug, Clone, Copy)]
    pub struct ProgramFlags: u32 {
        const EXECUTE = 0x00000001;
        const WRITE = 0x00000002;
        const READ = 0x00000004;
    }
}

impl Display for ProgramFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
