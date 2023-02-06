use self::cluster::ClustersReader;
use self::directory::{Directory, Item};
use self::entries::{ClusterAllocation, EntriesReader, EntryType, FileEntry};
use self::fat::Fat;
use self::file::File;
use self::image::Image;
use self::param::Params;
use std::io::{Read, Seek};
use std::sync::Arc;
use thiserror::Error;
use util::mem::{read_u16_le, read_u32_le, read_u8};

pub mod cluster;
pub mod directory;
pub mod entries;
pub mod fat;
pub mod file;
pub mod image;
pub mod param;

/// Represents an opened exFAT.
///
/// This implementation follows the official specs
/// https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification.
pub struct ExFat<I: Read + Seek> {
    volume_label: Option<String>,
    items: Vec<Item<I>>,
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
        let params = Params {
            fat_offset: unsafe { read_u32_le(boot, 80) } as u64,
            fat_length: unsafe { read_u32_le(boot, 84) } as u64,
            cluster_heap_offset: unsafe { read_u32_le(boot, 88) } as u64,
            cluster_count: unsafe { read_u32_le(boot, 92) } as usize,
            first_cluster_of_root_directory: unsafe { read_u32_le(boot, 96) } as usize,
            volume_flags: unsafe { read_u16_le(boot, 106) }.into(),
            bytes_per_sector: {
                let v = unsafe { read_u8(boot, 108) };

                if (9..=12).contains(&v) {
                    1u64 << v
                } else {
                    return Err(OpenError::InvalidBytesPerSectorShift);
                }
            },
            sectors_per_cluster: {
                let v = unsafe { read_u8(boot, 109) };

                // No need to check if subtraction is underflow because we already checked for the
                // valid value on the above.
                if v <= (25 - unsafe { read_u8(boot, 108) }) {
                    1u64 << v
                } else {
                    return Err(OpenError::InvalidSectorsPerClusterShift);
                }
            },
            number_of_fats: {
                let v = unsafe { read_u8(boot, 110) };

                if v == 1 || v == 2 {
                    v
                } else {
                    return Err(OpenError::InvalidNumberOfFats);
                }
            },
        };

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

        // Create a entries reader for the root directory.
        let root_cluster = params.first_cluster_of_root_directory;
        let mut reader = match ClustersReader::new(&params, &fat, &mut image, root_cluster, None) {
            Ok(v) => EntriesReader::new(v),
            Err(e) => return Err(OpenError::CreateClustersReaderFailed(e)),
        };

        // Load root directory.
        let mut allocation_bitmaps: [Option<ClusterAllocation>; 2] = [None, None];
        let mut upcase_table: Option<UpcaseTableEntry> = None;
        let mut volume_label: Option<String> = None;
        let mut files: Vec<FileEntry> = Vec::new();

        loop {
            // Read primary entry.
            let entry = match reader.read() {
                Ok(v) => v,
                Err(e) => return Err(OpenError::ReadEntryFailed(e)),
            };

            // Check entry type.
            let ty = entry.ty();

            if !ty.is_regular() {
                break;
            } else if ty.type_category() != EntryType::PRIMARY {
                return Err(OpenError::NotPrimaryEntry(entry.index(), entry.cluster()));
            }

            // Parse primary entry.
            match (ty.type_importance(), ty.type_code()) {
                (EntryType::CRITICAL, 1) => {
                    // Get next index.
                    let index = if allocation_bitmaps[1].is_some() {
                        return Err(OpenError::TooManyAllocationBitmap);
                    } else if allocation_bitmaps[0].is_some() {
                        1
                    } else {
                        0
                    };

                    // Load fields.
                    let data = entry.data();
                    let bitmap_flags = data[1] as usize;

                    if (bitmap_flags & 1) != index {
                        return Err(OpenError::WrongAllocationBitmap);
                    }

                    allocation_bitmaps[index] = match ClusterAllocation::load(&entry) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            return Err(OpenError::ReadClusterAllocationFailed(
                                entry.index(),
                                entry.cluster(),
                                e,
                            ));
                        }
                    };
                }
                (EntryType::CRITICAL, 2) => {
                    // Check if more than one up-case table.
                    if upcase_table.is_some() {
                        return Err(OpenError::MultipleUpcaseTable);
                    }

                    // Load fields.
                    let data = entry.data().as_ptr();
                    let checksum = unsafe { read_u32_le(data, 4) };
                    let data = match ClusterAllocation::load(&entry) {
                        Ok(v) => v,
                        Err(e) => {
                            return Err(OpenError::ReadClusterAllocationFailed(
                                entry.index(),
                                entry.cluster(),
                                e,
                            ));
                        }
                    };

                    upcase_table = Some(UpcaseTableEntry { checksum, data });
                }
                (EntryType::CRITICAL, 3) => {
                    // Check if more than one volume label.
                    if volume_label.is_some() {
                        return Err(OpenError::MultipleVolumeLabel);
                    }

                    // Load fields.
                    let data = entry.data();
                    let character_count = data[1] as usize;

                    if character_count > 11 {
                        return Err(OpenError::InvalidVolumeLabel);
                    }

                    volume_label = Some(String::from_utf16_lossy(util::slice::from_bytes(
                        &data[2..(2 + character_count * 2)],
                    )));
                }
                (EntryType::CRITICAL, 5) => match FileEntry::load(entry, &mut reader) {
                    Ok(v) => files.push(v),
                    Err(e) => return Err(OpenError::LoadFileEntryFailed(e)),
                },
                _ => return Err(OpenError::UnknownEntry(entry.index(), entry.cluster())),
            }
        }

        drop(reader);

        // Check allocation bitmap count.
        if params.number_of_fats == 2 {
            if allocation_bitmaps[1].is_none() {
                return Err(OpenError::NoAllocationBitmap);
            }
        } else if allocation_bitmaps[0].is_none() {
            return Err(OpenError::NoAllocationBitmap);
        }

        // Check Up-case Table.
        if upcase_table.is_none() {
            return Err(OpenError::NoUpcaseTable);
        }

        // Encapsulate the image.
        let image = Arc::new(Image::new(image, params, fat));

        // Construct root items.
        let mut items: Vec<Item<I>> = Vec::with_capacity(files.len());

        for file in files {
            let attrs = file.attributes;

            // Check if directory.
            let item = if attrs.is_directory() {
                Item::Directory(Directory::new(image.clone()))
            } else {
                Item::File(File::new(image.clone()))
            };

            items.push(item);
        }

        Ok(Self {
            volume_label,
            items,
        })
    }

    pub fn volume_label(&self) -> Option<&str> {
        self.volume_label.as_deref()
    }
}

impl<I: Read + Seek> IntoIterator for ExFat<I> {
    type Item = Item<I>;
    type IntoIter = std::vec::IntoIter<Item<I>>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Represents FileAttributes in the File Directory Entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FileAttributes(u16);

impl FileAttributes {
    pub fn is_read_only(self) -> bool {
        (self.0 & 0x0001) != 0
    }

    pub fn is_hidden(self) -> bool {
        (self.0 & 0x0002) != 0
    }

    pub fn is_system(self) -> bool {
        (self.0 & 0x0004) != 0
    }

    pub fn is_directory(self) -> bool {
        (self.0 & 0x0010) != 0
    }

    pub fn is_archive(self) -> bool {
        (self.0 & 0x0020) != 0
    }
}

/// Represents an Up-case Table Directory Entry.
struct UpcaseTableEntry {
    checksum: u32,
    data: ClusterAllocation,
}

/// Errors for [`open()`][ExFat::open()].
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

    #[error("cannot create a clusters reader")]
    CreateClustersReaderFailed(#[source] cluster::NewError),

    #[error("cannot read a directory entry")]
    ReadEntryFailed(#[source] entries::ReaderError),

    #[error("directory entry #{0} on cluster #{1} is not a primary entry")]
    NotPrimaryEntry(usize, usize),

    #[error("more than 2 allocation bitmaps exists in the root directory")]
    TooManyAllocationBitmap,

    #[error("allocation bitmap in the root directory is not for its corresponding FAT")]
    WrongAllocationBitmap,

    #[error("multiple up-case table exists in the root directory")]
    MultipleUpcaseTable,

    #[error("multiple volume label exists in the root directory")]
    MultipleVolumeLabel,

    #[error("invalid volume label")]
    InvalidVolumeLabel,

    #[error("cannot load file entry in the root directory")]
    LoadFileEntryFailed(#[source] entries::FileEntryError),

    #[error("cannot read cluster allocation for entry #{0} on cluster #{1}")]
    ReadClusterAllocationFailed(usize, usize, #[source] entries::ClusterAllocationError),

    #[error("unknown directory entry #{0} on cluster #{1}")]
    UnknownEntry(usize, usize),

    #[error("no Allocation Bitmap available for active FAT")]
    NoAllocationBitmap,

    #[error("no Up-case Table available")]
    NoUpcaseTable,
}
