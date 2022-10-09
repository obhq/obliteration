use crate::fs::file::File;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{read_array, read_u16_le, uninit};

// https://www.psdevwiki.com/ps4/SELF_File_Format
pub struct Process {}

impl Process {
    pub(super) fn load(mut bin: File) -> Result<Self, LoadError> {
        // Read header.
        let mut hdr: [u8; 32] = uninit();

        bin.read_exact(&mut hdr)?;

        let hdr = hdr.as_ptr();

        // Check magic.
        // Kyty also checking if Category = 0x01 & Program Type = 0x01 & Padding = 0x00.
        // Let's check only magic for now until something is broken.
        let magic: [u8; 8] = read_array(hdr, 0x00);
        let unknown = read_u16_le(hdr, 0x1a);

        if magic != [0x4f, 0x15, 0x3d, 0x1d, 0x00, 0x01, 0x01, 0x12] || unknown != 0x22 {
            return Err(LoadError::InvalidMagic);
        }

        // Load header fields.
        let segments = read_u16_le(hdr, 0x18);

        // Load segment headers.
        for _ in 0..segments {
            let mut hdr: [u8; 32] = uninit();

            bin.read_exact(&mut hdr)?;
        }

        Ok(Self {})
    }
}

#[derive(Debug)]
pub enum LoadError {
    IoFailed(std::io::Error),
    InvalidMagic,
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IoFailed(_) => f.write_str("I/O failed"),
            Self::InvalidMagic => f.write_str("invalid magic"),
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(v: std::io::Error) -> Self {
        Self::IoFailed(v)
    }
}
