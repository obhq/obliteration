use crate::errno::{strerror, Errno};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;

/// Error of each syscall.
#[derive(Debug)]
pub enum SysErr {
    Raw(NonZeroI32),
    Object(Box<dyn Errno>),
}

impl SysErr {
    pub fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Raw(v) => *v,
            Self::Object(v) => v.errno(),
        }
    }
}

impl From<Box<dyn Errno>> for SysErr {
    fn from(value: Box<dyn Errno>) -> Self {
        Self::Object(value)
    }
}

impl<T: Errno + 'static> From<T> for SysErr {
    fn from(value: T) -> Self {
        Self::Object(Box::new(value))
    }
}

impl Error for SysErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Raw(_) => None,
            Self::Object(e) => e.source(),
        }
    }
}

impl Display for SysErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(v) => f.write_str(strerror(*v)),
            Self::Object(e) => Display::fmt(&e, f),
        }
    }
}
