use std::fmt::{Display, Formatter};

pub struct Program64 {
    ty: ProgramType,
    offset: u64,
    file_size: u64,
}

impl Program64 {
    pub(super) fn new(ty: ProgramType, offset: u64, file_size: u64) -> Self {
        Self {
            ty,
            offset,
            file_size,
        }
    }

    pub fn ty(&self) -> ProgramType {
        self.ty
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
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
