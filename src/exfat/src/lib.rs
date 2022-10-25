use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::read_array;

pub struct ExFat<I: Read> {
    image: I,
    boot_sector: BootSector,
}

impl<I: Read> ExFat<I> {
    pub fn open(mut image: I) -> Result<Self, OpenError> {
        let boot_sector = Self::read_boot_sector(&mut image)?;

        Ok(Self { image, boot_sector })
    }

    fn read_boot_sector(image: &mut I) -> Result<BootSector, OpenError> {
        // Load sector.
        let sector: [u8; 512] = match util::io::read_array(image) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::ReadBootSectorFailed(e)),
        };

        let sector = sector.as_ptr();

        // Check type.
        let file_system_name: [u8; 8] = read_array(sector, 3);

        if &file_system_name != b"EXFAT   " {
            return Err(OpenError::NotExFat);
        }

        let must_be_zero: [u8; 53] = read_array(sector, 11);

        if !must_be_zero.iter().all(|&b| b == 0) {
            return Err(OpenError::NotExFat);
        }

        Ok(BootSector {})
    }
}

pub struct BootSector {}

#[derive(Debug)]
pub enum OpenError {
    ReadBootSectorFailed(std::io::Error),
    NotExFat,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBootSectorFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadBootSectorFailed(_) => f.write_str("cannot read boot sector"),
            Self::NotExFat => f.write_str("image is not exFAT"),
        }
    }
}
