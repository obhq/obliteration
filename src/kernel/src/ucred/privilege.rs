use std::fmt::{Display, Formatter};

/// Privilege identifier.
///
/// See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/sys/priv.h for standard
/// FreeBSD privileges.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Privilege(i32);

impl Privilege {
    pub const SCE686: Self = Self(686);
}

impl Display for Privilege {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::SCE686 => f.write_str("SCE686"),
            v => v.0.fmt(f),
        }
    }
}
