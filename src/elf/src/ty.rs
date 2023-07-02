use std::fmt::{Display, Formatter};

/// Type of (S)ELF file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileType(u16);

impl FileType {
    pub const ET_EXEC: Self = Self(0x0002);
    pub const ET_SCE_EXEC: Self = Self(0xfe00);
    pub const ET_SCE_REPLAY_EXEC: Self = Self(0xfe01);
    pub const ET_SCE_DYNEXEC: Self = Self(0xfe10);
    pub const ET_SCE_DYNAMIC: Self = Self(0xfe18);

    pub fn new(v: u16) -> Self {
        Self(v)
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::ET_EXEC => f.write_str("ET_EXEC"),
            Self::ET_SCE_EXEC => f.write_str("ET_SCE_EXEC"),
            Self::ET_SCE_REPLAY_EXEC => f.write_str("ET_SCE_REPLAY_EXEC"),
            Self::ET_SCE_DYNEXEC => f.write_str("ET_SCE_DYNEXEC"),
            Self::ET_SCE_DYNAMIC => f.write_str("ET_SCE_DYNAMIC"),
            _ => write!(f, "{:#06x}", self.0),
        }
    }
}
