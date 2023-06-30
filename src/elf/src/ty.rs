/// Type of (S)ELF file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileType(u16);

impl FileType {
    pub const ET_SCE_REPLAY_EXEC: Self = Self(0xfe01);
    pub const ET_SCE_DYNEXEC: Self = Self(0xfe10);

    pub fn new(v: u16) -> Self {
        Self(v)
    }
}
