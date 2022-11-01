use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::uninit;
use util::slice::as_mut_bytes;

// https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification
pub struct ExFat<I: Read> {
    image: I,
}

impl<I: Read> ExFat<I> {
    pub fn open(mut image: I, boot_checksum: bool) -> Result<Self, OpenError> {
        // Read main boot region.
        if let Err(e) = Self::read_boot_region(&mut image, boot_checksum) {
            return Err(match e {
                BootRegionError::IoFailed(e) => OpenError::ReadMainBootFailed(e),
                BootRegionError::NotExFat => OpenError::NotExFat,
                BootRegionError::InvalidChecksum => OpenError::InvalidMainBootChecksum,
            });
        }

        Ok(Self { image })
    }

    fn read_boot_region(image: &mut I, do_checksum: bool) -> Result<(), BootRegionError> {
        // Read all sectors except checksum.
        let sectors: [u8; 512 * 11] = match util::io::read_array(image) {
            Ok(v) => v,
            Err(e) => return Err(BootRegionError::IoFailed(e)),
        };

        // Check type.
        if &sectors[3..11] != b"EXFAT   " || !sectors[11..64].iter().all(|&b| b == 0) {
            return Err(BootRegionError::NotExFat);
        }

        // Read checksum.
        let mut checksums: [u32; 512 / 4] = uninit();

        if let Err(e) = image.read_exact(as_mut_bytes(&mut checksums)) {
            return Err(BootRegionError::IoFailed(e));
        }

        // Do checksum.
        if do_checksum && !Self::checksum_boot_region(&checksums, &sectors) {
            return Err(BootRegionError::InvalidChecksum);
        }

        Ok(())
    }

    fn checksum_boot_region(checksums: &[u32], sectors: &[u8]) -> bool {
        let mut checksum: u32 = 0;

        for i in 0..sectors.len() {
            if i == 106 || i == 107 || i == 112 {
                continue;
            }

            checksum = (checksum >> 1)
                + sectors[i] as u32
                + if (checksum & 1) != 0 { 0x80000000 } else { 0 };
        }

        for &expect in checksums {
            if expect != checksum {
                return false;
            }
        }

        true
    }
}

#[derive(Debug)]
pub enum OpenError {
    ReadMainBootFailed(std::io::Error),
    NotExFat,
    InvalidMainBootChecksum,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadMainBootFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadMainBootFailed(_) => f.write_str("cannot read main boot region"),
            Self::NotExFat => f.write_str("image is not exFAT"),
            Self::InvalidMainBootChecksum => f.write_str("invalid checksum for main boot region"),
        }
    }
}

enum BootRegionError {
    IoFailed(std::io::Error),
    NotExFat,
    InvalidChecksum,
}
