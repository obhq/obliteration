use self::directory::entry::EntrySet;
use self::fat::Fat;
use self::param::Params;
use std::io::{Read, Seek};
use std::sync::Arc;
use thiserror::Error;
use util::mem::{read_u16_le, read_u32_le, read_u8};

pub mod cluster;
pub mod directory;
pub mod fat;
pub mod param;

// https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification
pub struct ExFat<I: Read + Seek> {
    image: I,
    params: Arc<Params>,
    fat: Fat,
    volume_label: Option<String>,
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
            fat_length: read_u32_le(boot, 84) as u64,
            cluster_heap_offset: read_u32_le(boot, 88) as u64,
            cluster_count: read_u32_le(boot, 92) as usize,
            first_cluster_of_root_directory: read_u32_le(boot, 96) as usize,
            volume_flags: read_u16_le(boot, 106).into(),
            bytes_per_sector: {
                let v = read_u8(boot, 108);

                if (9..=12).contains(&v) {
                    1u64 << v
                } else {
                    return Err(OpenError::InvalidBytesPerSectorShift);
                }
            },
            sectors_per_cluster: {
                let v = read_u8(boot, 109);

                // No need to check if subtraction is underflow because we already checked for the
                // valid value on the above.
                if v <= (25 - read_u8(boot, 108)) {
                    1u64 << v
                } else {
                    return Err(OpenError::InvalidSectorsPerClusterShift);
                }
            },
            number_of_fats: {
                let v = read_u8(boot, 110);

                if v == 1 || v == 2 {
                    v
                } else {
                    return Err(OpenError::InvalidNumberOfFats);
                }
            },
        });

        // Read FAT region.
        let active_fat = params.volume_flags.active_fat();
        let fat = if active_fat == 0 || params.number_of_fats == 2 {
            match Fat::load(&params, &mut image, active_fat) {
                Ok(v) => v,
                Err(e) => return Err(OpenError::ReadFatRegionFailed(e)),
            }
        } else {
            return Err(OpenError::InvalidNumberOfFats);
        };

        // Load root directory.
        let root_cluster = params.first_cluster_of_root_directory;
        let entries = match EntrySet::load(&params, &fat, &mut image, root_cluster) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::ReadRootFailed(e)),
        };

        // Check allocation bitmap count.
        if params.number_of_fats == 2 {
            if entries.allocation_bitmaps[1].is_none() {
                return Err(OpenError::NoAllocationBitmap);
            }
        } else if entries.allocation_bitmaps[0].is_none() {
            return Err(OpenError::NoAllocationBitmap);
        }

        Ok(Self {
            image,
            params,
            fat,
            volume_label: entries.volume_label,
        })
    }

    pub fn volume_label(&self) -> Option<&str> {
        self.volume_label.as_deref()
    }
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot read main boot region")]
    ReadMainBootFailed(#[source] std::io::Error),

    #[error("image is not exFAT")]
    NotExFat,

    #[error("invalid BytesPerSectorShift")]
    InvalidBytesPerSectorShift,

    #[error("invalid SectorsPerClusterShift")]
    InvalidSectorsPerClusterShift,

    #[error("invalid NumberOfFats")]
    InvalidNumberOfFats,

    #[error("cannot read FAT region")]
    ReadFatRegionFailed(#[source] fat::LoadError),

    #[error("cannot read root directory")]
    ReadRootFailed(#[source] directory::entry::LoadEntriesError),

    #[error("no Allocation Bitmap available for active FAT")]
    NoAllocationBitmap,
}
