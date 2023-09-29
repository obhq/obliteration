use std::fmt::{Display, Formatter};

/// Privilege identifier.
///
/// See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/sys/priv.h for standard
/// FreeBSD privileges.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Privilege(i32);

impl Privilege {
    pub const SCE680: Self = Self(680);
    pub const SCE683: Self = Self(683);
    pub const SCE686: Self = Self(686);
}

impl Display for Privilege {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::SCE680 => f.write_str("SCE680"),
            Self::SCE683 => f.write_str("SCE683"),
            Self::SCE686 => f.write_str("SCE686"),
            v => v.0.fmt(f),
        }
    }
}
