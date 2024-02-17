use crate::time::TimeSpec;

/// An implementation of the `stat` structure.
#[repr(C)]
pub struct Stat {
    pub dev: i32,
    pub ino: u32,
    pub mode: u16,
    pub nlink: u16,
    pub uid: u32,
    pub gid: u32,
    pub rdev: i32,
    pub atime: TimeSpec,
    pub mtime: TimeSpec,
    pub ctime: TimeSpec,
    pub size: u64,
    pub block_count: u64,
    pub block_size: u32,
    pub flags: u32,
    pub gen: u32,
    _spare: i32,
    pub birthtime: TimeSpec,
}

impl Stat {
    /// This is what would happen when calling `bzero` on a `stat` structure.
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
