use crate::fs::file::File;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{read_array, read_u16_le, read_u8, uninit};

// https://www.psdevwiki.com/ps4/SELF_File_Format
pub struct Executable {
    class: Class,
}

impl Executable {
    pub fn load(mut file: File) -> Result<Self, LoadError> {
        // Read SELF header.
        let mut hdr: [u8; 32] = uninit();

        file.read_exact(&mut hdr)?;

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

            file.read_exact(&mut hdr)?;
        }

        // Read ELF header.
        let mut hdr: [u8; 64] = uninit();

        file.read_exact(&mut hdr)?;

        // Load ELF header.
        let hdr = hdr.as_ptr();
        let class = Class(read_u8(hdr, 0x04));

        if class != Class::THIRTY_TWO_BITS && class != Class::SIXTY_FOUR_BITS {
            return Err(LoadError::InvalidClass);
        }

        Ok(Self { class })
    }

    pub fn class(&self) -> Class {
        self.class
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Class(u8);

impl Class {
    pub const THIRTY_TWO_BITS: Class = Class(1);
    pub const SIXTY_FOUR_BITS: Class = Class(2);
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            &Self::THIRTY_TWO_BITS => f.write_str("32-bits"),
            &Self::SIXTY_FOUR_BITS => f.write_str("64-bits"),
            _ => panic!("Unknown class {}.", self.0),
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    IoFailed(std::io::Error),
    InvalidMagic,
    InvalidClass,
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
            Self::InvalidClass => f.write_str("invalid class"),
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(v: std::io::Error) -> Self {
        Self::IoFailed(v)
    }
}
