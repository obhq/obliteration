use crate::errno::{strerror, AsErrno};
use crate::Errno;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Error of each syscall.
#[derive(Debug)]
pub enum SysErr {
    Raw(Errno),
    Object(Box<dyn AsErrno>),
}

impl SysErr {
    pub fn errno(&self) -> Errno {
        match self {
            Self::Raw(v) => *v,
            Self::Object(v) => v.errno(),
        }
    }
}

impl From<Box<dyn AsErrno>> for SysErr {
    fn from(value: Box<dyn AsErrno>) -> Self {
        Self::Object(value)
    }
}

impl<T: AsErrno + 'static> From<T> for SysErr {
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
