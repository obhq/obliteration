use crate::fs::MountOpt;

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

impl From<Uid> for MountOpt {
    fn from(v: Uid) -> Self {
        Self::Uid(v)
    }
}

impl TryFrom<MountOpt> for Uid {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::Uid(v) => Ok(v),
            _ => Err(()),
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

impl From<Gid> for MountOpt {
    fn from(v: Gid) -> Self {
        Self::Gid(v)
    }
}

impl TryFrom<MountOpt> for Gid {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::Gid(v) => Ok(v),
            _ => Err(()),
        }
    }
}
