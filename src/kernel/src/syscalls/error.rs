use crate::errno::{strerror, Errno};
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;

/// Error result of each syscall.
#[derive(Debug)]
pub enum Error {
    Raw(NonZeroI32),
    Object(Box<dyn Errno>),
}

impl Error {
    pub fn errno(&self) -> NonZeroI32 {
        match self {
            Error::Raw(v) => *v,
            Error::Object(v) => v.errno(),
        }
    }
}

impl<T: Errno + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Self::Object(Box::new(value))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(v) => f.write_str(strerror(*v)),
            Self::Object(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Raw(_) => None,
            Error::Object(e) => e.source(),
        }
    }
}
