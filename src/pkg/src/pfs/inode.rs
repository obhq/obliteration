use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::uninit;

pub struct Inode {}

impl Inode {
    pub fn read_unsigned<F: Read>(from: &mut F) -> Result<Self, ReadError> {
        let raw: [u8; 168] = Self::read_raw(from)?;

        Ok(Self {})
    }

    pub fn read_signed<F: Read>(from: &mut F) -> Result<Self, ReadError> {
        let raw: [u8; 712] = Self::read_raw(from)?;

        Ok(Self {})
    }

    fn read_raw<const L: usize, F: Read>(from: &mut F) -> Result<[u8; L], ReadError> {
        let mut raw: [u8; L] = uninit();

        if let Err(e) = from.read_exact(&mut raw) {
            return Err(if e.kind() == std::io::ErrorKind::UnexpectedEof {
                ReadError::TooSmall
            } else {
                ReadError::IoFailed(e)
            });
        }

        Ok(raw)
    }
}

#[derive(Debug)]
pub enum ReadError {
    IoFailed(std::io::Error),
    TooSmall,
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IoFailed(_) => f.write_str("I/O failed"),
            Self::TooSmall => f.write_str("data too small"),
        }
    }
}
