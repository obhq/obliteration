use crate::errno::Errno;
use std::num::NonZeroI32;

/// Error result of each syscall.
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
