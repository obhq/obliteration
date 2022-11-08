use self::fat::Fat;
use self::param::Params;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};
use std::sync::Arc;
use util::mem::{read_u32_le, read_u8};

pub mod cluster;
pub mod fat;
pub mod param;

// https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification
pub struct ExFat<I: Read + Seek> {
    params: Arc<Params>,
    fat: Fat,
    image: I,
}

impl<I: Read + Seek> ExFat<I> {
    pub fn open(mut image: I) -> Result<Self, OpenError> {
        // Read boot sector.
        let boot: [u8; 512] = match util::io::read_array(&mut image) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::ReadMainBootFailed(e)),
        };

        // Check type.
        if &boot[3..11] != b"EXFAT   " || !boot[11..64].iter().all(|&b| b == 0) {
            return Err(OpenError::NotExFat);
        }

        // Load fields.
        let boot = boot.as_ptr();
        let params = Arc::new(Params {
            fat_offset: read_u32_le(boot, 80) as u64,
            cluster_heap_offset: read_u32_le(boot, 88) as u64,
            cluster_count: read_u32_le(boot, 92) as usize,
            first_cluster_of_root_directory: read_u32_le(boot, 96) as usize,
            bytes_per_sector: match read_u8(boot, 108) {
                v if v >= 9 && v <= 12 => 1u64 << v,
                _ => return Err(OpenError::InvalidBytesPerSectorShift),
            },
            sectors_per_cluster: match read_u8(boot, 109) {
                v if v <= 25 - read_u8(boot, 108) => 1u64 << v,
                _ => return Err(OpenError::InvalidSectorsPerClusterShift),
            },
        });

        // Read FAT region.
        let fat = match Fat::load(params.clone(), &mut image) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::ReadFatRegionFailed(e)),
        };

        Ok(Self { params, fat, image })
    }
}

#[derive(Debug)]
pub enum OpenError {
    ReadMainBootFailed(std::io::Error),
    NotExFat,
    InvalidBytesPerSectorShift,
    InvalidSectorsPerClusterShift,
    ReadFatRegionFailed(fat::LoadError),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadMainBootFailed(e) => Some(e),
            Self::ReadFatRegionFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadMainBootFailed(_) => f.write_str("cannot read main boot region"),
            Self::NotExFat => f.write_str("image is not exFAT"),
            Self::InvalidBytesPerSectorShift => f.write_str("invalid BytesPerSectorShift"),
            Self::InvalidSectorsPerClusterShift => f.write_str("invalid SectorsPerClusterShift"),
            Self::ReadFatRegionFailed(_) => f.write_str("cannot read FAT region"),
        }
    }
}
