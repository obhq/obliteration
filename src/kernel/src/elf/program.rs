use std::fmt::{Display, Formatter};

pub struct Program {
    ty: ProgramType,
    flags: ProgramFlags,
    offset: u64,
    virtual_addr: usize,
    file_size: u64,
    memory_size: usize,
    aligment: usize,
}

impl Program {
    pub(super) fn new(
        ty: ProgramType,
        flags: ProgramFlags,
        offset: u64,
        virtual_addr: usize,
        file_size: u64,
        memory_size: usize,
        aligment: usize,
    ) -> Self {
        Self {
            ty,
            flags,
            offset,
            virtual_addr,
            file_size,
            memory_size,
            aligment,
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

    pub fn virtual_addr(&self) -> usize {
        self.virtual_addr
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn memory_size(&self) -> usize {
        self.memory_size
    }

    pub fn aligned_size(&self) -> usize {
        if self.aligment != 0 {
            // FIXME: Refactor this for readability.
            (self.memory_size + (self.aligment - 1)) & !(self.aligment - 1)
        } else {
            self.memory_size
        }
    }

    pub fn aligment(&self) -> usize {
        self.aligment
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
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
}

impl From<u32> for ProgramType {
    fn from(v: u32) -> Self {
        ProgramType(v)
    }
}

impl Display for ProgramType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            &Self::PT_LOAD => f.write_str("PT_LOAD"),
            &Self::PT_DYNAMIC => f.write_str("PT_DYNAMIC"),
            &Self::PT_INTERP => f.write_str("PT_INTERP"),
            &Self::PT_TLS => f.write_str("PT_TLS"),
            &Self::PT_SCE_DYNLIBDATA => f.write_str("PT_SCE_DYNLIBDATA"),
            &Self::PT_SCE_PROCPARAM => f.write_str("PT_SCE_PROCPARAM"),
            &Self::PT_SCE_MODULE_PARAM => f.write_str("PT_SCE_MODULE_PARAM"),
            &Self::PT_SCE_RELRO => f.write_str("PT_SCE_RELRO"),
            &Self::PT_SCE_COMMENT => f.write_str("PT_SCE_COMMENT"),
            &Self::PT_SCE_VERSION => f.write_str("PT_SCE_VERSION"),
            &Self::PT_GNU_EH_FRAME => f.write_str("PT_GNU_EH_FRAME"),
            t => write!(f, "{:#010x}", t.0),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ProgramFlags(u32);

#[allow(dead_code)]
impl ProgramFlags {
    pub const READ: ProgramFlags = ProgramFlags(1);
    pub const WRITE: ProgramFlags = ProgramFlags(2);
    pub const EXECUTE: ProgramFlags = ProgramFlags(4);
    pub const GPU_EXECUTE: ProgramFlags = ProgramFlags(8);
    pub const GPU_READ: ProgramFlags = ProgramFlags(16);
    pub const GPU_WRITE: ProgramFlags = ProgramFlags(32);

    pub fn is_readable(self) -> bool {
        (self.0 & 1) != 0
    }

    pub fn is_writable(self) -> bool {
        (self.0 & 2) != 0
    }

    pub fn is_executable(self) -> bool {
        (self.0 & 4) != 0
    }

    pub fn is_gpu_executable(self) -> bool {
        (self.0 & 8) != 0
    }

    pub fn is_gpu_readable(self) -> bool {
        (self.0 & 16) != 0
    }

    pub fn is_gpu_writable(self) -> bool {
        (self.0 & 32) != 0
    }
}

impl From<u32> for ProgramFlags {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl Display for ProgramFlags {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        // TODO: Show constant name.
        write!(f, "{:#010x}", self.0)
    }
}
