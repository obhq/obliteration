use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{new_buffer, read_u32_le, uninit};

pub(super) struct Dirent {
    ino: usize,
    ty: u32,
    entsize: usize,
    name: Vec<u8>,
}

impl Dirent {
    pub const FILE: u32 = 2;
    pub const DIRECTORY: u32 = 3;
    pub const SELF: u32 = 4;
    pub const PARENT: u32 = 5;

    pub fn read<F: Read>(from: &mut F) -> Result<Self, ReadError> {
        // Read static sized fields.
        let mut data: [u8; 16] = uninit();

        from.read_exact(&mut data)?;

        let raw = data.as_ptr();
        let entsize = read_u32_le(raw, 0x0c) as usize;

        if entsize == 0 {
            return Err(ReadError::EndOfEntry);
        }

        let ino = read_u32_le(raw, 0x00) as usize;
        let ty = read_u32_le(raw, 0x04);
        let namelen = read_u32_le(raw, 0x08) as usize;

        // Read name.
        let mut name = new_buffer(namelen);

        from.read_exact(&mut name)?;

        Ok(Self {
            ino,
            ty,
            entsize,
            name,
        })
    }

    pub fn inode(&self) -> usize {
        self.ino
    }

    pub fn ty(&self) -> u32 {
        self.ty
    }

    pub fn take_name(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.name)
    }

    /// This method **MUST** be called before [`take_name`] otherwise the returned value will be incorrect.
    pub fn padding_size(&self) -> usize {
        self.entsize - 16 - self.name.len()
    }
}

#[derive(Debug)]
pub enum ReadError {
    IoFailed(std::io::Error),
    TooSmall,
    EndOfEntry,
}

impl From<std::io::Error> for ReadError {
    fn from(v: std::io::Error) -> Self {
        if v.kind() == std::io::ErrorKind::UnexpectedEof {
            ReadError::TooSmall
        } else {
            ReadError::IoFailed(v)
        }
    }
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
            Self::EndOfEntry => f.write_str("end of entry"),
        }
    }
}
