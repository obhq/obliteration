use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use util::mem::{new_buffer, read_u32_le, read_u8};
use util::slice::as_mut_bytes;

// https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification
pub struct ExFat<I: Read + Seek> {
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
        let fat_offset = read_u32_le(boot, 80) as u64; // in sector
        let cluster_count = read_u32_le(boot, 92) as usize;
        let bytes_per_sector = match 1u64.checked_shl(read_u8(boot, 108) as _) {
            Some(v) => v,
            None => return Err(OpenError::InvalidBytesPerSectorShift),
        };

        // Read FAT region.
        let offset = match fat_offset.checked_mul(bytes_per_sector) {
            Some(v) => v,
            None => return Err(OpenError::InvalidFatOffset),
        };

        match image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(OpenError::InvalidFatOffset);
                }
            }
            Err(e) => return Err(OpenError::ReadFatRegionFailed(e)),
        }

        let mut fat_entries: Vec<u32> = new_buffer(cluster_count + 2);

        if let Err(e) = image.read_exact(as_mut_bytes(&mut fat_entries)) {
            return Err(OpenError::ReadFatRegionFailed(e));
        }

        // Check first fat.
        if fat_entries[0] != 0xfffffff8 {
            return Err(OpenError::InvalidFatEntry(0));
        }

        Ok(Self { image })
    }
}

#[derive(Debug)]
pub enum OpenError {
    ReadMainBootFailed(std::io::Error),
    NotExFat,
    InvalidBytesPerSectorShift,
    InvalidFatOffset,
    ReadFatRegionFailed(std::io::Error),
    InvalidFatEntry(usize),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadMainBootFailed(e) | Self::ReadFatRegionFailed(e) => Some(e),
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
            Self::InvalidFatOffset => f.write_str("invalid FatOffset"),
            Self::ReadFatRegionFailed(_) => f.write_str("cannot read FAT region"),
            Self::InvalidFatEntry(i) => write!(f, "FAT entry #{} is not valid", i),
        }
    }
}
