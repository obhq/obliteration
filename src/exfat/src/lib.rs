use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{read_array, uninit};

pub struct ExFat<I: Read> {
    image: I,
    boot_sector: BootSector,
}

impl<I: Read> ExFat<I> {
    pub fn open(mut image: I) -> Result<Self, OpenError> {
        let boot_sector = Self::read_boot_sector(&mut image)?;
        Self::read_extended_boot_sectors(&mut image)?;
        Self::read_oem_parameters(&mut image)?;
        Self::read_reserved(&mut image)?;

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

    fn read_extended_boot_sectors(image: &mut I) -> Result<(), OpenError> {
        for i in 0..8usize {
            let mut sector: [u8; 512] = uninit();

            if let Err(e) = image.read_exact(&mut sector) {
                return Err(OpenError::ReadExtendedBootSectorsFailed(i, e));
            }
        }

        Ok(())
    }

    fn read_oem_parameters(image: &mut I) -> Result<(), OpenError> {
        let mut sector: [u8; 512] = uninit();

        if let Err(e) = image.read_exact(&mut sector) {
            return Err(OpenError::ReadOemParametersFailed(e));
        }

        Ok(())
    }

    fn read_reserved(image: &mut I) -> Result<(), OpenError> {
        let mut sector: [u8; 512] = uninit();

        if let Err(e) = image.read_exact(&mut sector) {
            return Err(OpenError::ReadReservedFailed(e));
        }

        Ok(())
    }
}

pub struct BootSector {}

#[derive(Debug)]
pub enum OpenError {
    ReadBootSectorFailed(std::io::Error),
    NotExFat,
    ReadExtendedBootSectorsFailed(usize, std::io::Error),
    ReadOemParametersFailed(std::io::Error),
    ReadReservedFailed(std::io::Error),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBootSectorFailed(e)
            | Self::ReadExtendedBootSectorsFailed(_, e)
            | Self::ReadOemParametersFailed(e)
            | Self::ReadReservedFailed(e) => Some(e),
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
        }
    }
}
