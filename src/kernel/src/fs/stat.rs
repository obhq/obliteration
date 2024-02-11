use crate::time::TimeSpec;

/// An implementation of the `stat` structure.
#[repr(C)]
pub struct Stat {
    dev: i32,
    ino: u32,
    pub mode: u16,
    nlink: u16,
    uid: u32,
    gid: u32,
    rdev: i32,
    atime: TimeSpec,
    mtime: TimeSpec,
    ctime: TimeSpec,
    size: i64,
    block_count: i64,
    pub block_size: u32,
    flags: u32,
    gen: u32,
    _spare: i32,
    birthtime: TimeSpec,
}

impl Stat {
    /// This is what would happen when calling `bzero` on a `stat` structure.
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
