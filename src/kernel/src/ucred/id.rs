/// An implementation of `uid_t`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uid(i32);

impl Uid {
    pub const ROOT: Self = Self(0);

    pub const fn new(v: i32) -> Option<Self> {
        if v >= 0 {
            Some(Self(v))
        } else {
            None
        }
    }
}

/// An implementation of `gid_t`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gid(i32);

impl Gid {
    pub const ROOT: Self = Self(0);

    pub const fn new(v: i32) -> Option<Self> {
        if v >= 0 {
            Some(Self(v))
        } else {
            None
        }
    }
}
