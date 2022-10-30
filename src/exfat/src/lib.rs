use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{read_array, uninit};
use util::slice::as_mut_bytes;

pub struct ExFat<I: Read> {
    image: I,
}

impl<I: Read> ExFat<I> {
    pub fn open(mut image: I, boot_checksum: bool) -> Result<Self, OpenError> {
        // Read main boot region.
        let mut sectors: Vec<u8> = Vec::with_capacity(512 * 11);

        Self::read_boot_sector(&mut image, &mut sectors)?;
        Self::read_extended_boot_sectors(&mut image, &mut sectors)?;
        Self::read_oem_parameters(&mut image, &mut sectors)?;
        Self::read_reserved(&mut image, &mut sectors)?;

        if !Self::read_boot_checksum(&mut image, &sectors)? && boot_checksum {
            return Err(OpenError::InvalidMainBootChecksum);
        }

        Ok(Self { image })
    }

    fn read_boot_sector(image: &mut I, sectors: &mut Vec<u8>) -> Result<(), OpenError> {
        // Load sector.
        let sector: [u8; 512] = match util::io::read_array(image) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::ReadBootSectorFailed(e)),
        };

        sectors.extend_from_slice(&sector);

        // Check type.
        let sector = sector.as_ptr();
        let file_system_name: [u8; 8] = read_array(sector, 3);

        if &file_system_name != b"EXFAT   " {
            return Err(OpenError::NotExFat);
        }

        let must_be_zero: [u8; 53] = read_array(sector, 11);

        if !must_be_zero.iter().all(|&b| b == 0) {
            return Err(OpenError::NotExFat);
        }

        Ok(())
    }

    fn read_extended_boot_sectors(image: &mut I, sectors: &mut Vec<u8>) -> Result<(), OpenError> {
        for i in 0..8usize {
            let mut sector: [u8; 512] = uninit();

            if let Err(e) = image.read_exact(&mut sector) {
                return Err(OpenError::ReadExtendedBootSectorsFailed(i, e));
            }

            sectors.extend_from_slice(&sector);
        }

        Ok(())
    }

    fn read_oem_parameters(image: &mut I, sectors: &mut Vec<u8>) -> Result<(), OpenError> {
        let mut sector: [u8; 512] = uninit();

        if let Err(e) = image.read_exact(&mut sector) {
            return Err(OpenError::ReadOemParametersFailed(e));
        }

        sectors.extend_from_slice(&sector);

        Ok(())
    }

    fn read_reserved(image: &mut I, sectors: &mut Vec<u8>) -> Result<(), OpenError> {
        let mut sector: [u8; 512] = uninit();

        if let Err(e) = image.read_exact(&mut sector) {
            return Err(OpenError::ReadReservedFailed(e));
        }

        sectors.extend_from_slice(&sector);

        Ok(())
    }

    fn read_boot_checksum(image: &mut I, sectors: &[u8]) -> Result<bool, OpenError> {
        // Read sector.
        let mut sector: [u32; 512 / 4] = uninit();

        if let Err(e) = image.read_exact(as_mut_bytes(&mut sector)) {
            return Err(OpenError::ReadBootChecksumFailed(e));
        }

        // Calculate checksum.
        let mut checksum: u32 = 0;

        for i in 0..sectors.len() {
            if i == 106 || i == 107 || i == 112 {
                continue;
            }
            checksum = (checksum >> 1)
                + sectors[i] as u32
                + if (checksum & 1) != 0 { 0x80000000 } else { 0 };
        }

        // Do checksum.
        for expect in sector {
            if expect != checksum {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[derive(Debug)]
pub enum OpenError {
    ReadBootSectorFailed(std::io::Error),
    NotExFat,
    ReadExtendedBootSectorsFailed(usize, std::io::Error),
    ReadOemParametersFailed(std::io::Error),
    ReadReservedFailed(std::io::Error),
    ReadBootChecksumFailed(std::io::Error),
    InvalidMainBootChecksum,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBootSectorFailed(e)
            | Self::ReadExtendedBootSectorsFailed(_, e)
            | Self::ReadOemParametersFailed(e)
            | Self::ReadReservedFailed(e)
            | Self::ReadBootChecksumFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadBootSectorFailed(_) => f.write_str("cannot read boot sector"),
            Self::NotExFat => f.write_str("image is not exFAT"),
            Self::ReadExtendedBootSectorsFailed(i, _) => {
                write!(f, "cannot read extended boot sectors #{}", i)
            }
            Self::ReadOemParametersFailed(_) => f.write_str("cannot read OEM parameters"),
            Self::ReadReservedFailed(_) => f.write_str("cannot read reserved region"),
            Self::ReadBootChecksumFailed(_) => f.write_str("cannot read boot checksum"),
            Self::InvalidMainBootChecksum => f.write_str("invalid checksum for main boot region"),
        }
    }
}
