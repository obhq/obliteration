use crate::cluster::ClusterReader;
use crate::fat::Fat;
use crate::param::Params;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};
use util::mem::{read_u32_le, read_u64_le, read_u8};

pub(crate) struct EntrySet {
    pub volume_label: Option<String>,
    pub allocation_bitmaps: [Option<DataDescriptor>; 2],
}

impl EntrySet {
    pub fn load<I: Read + Seek>(
        params: &Params,
        fat: &Fat,
        image: &mut I,
        first_cluster: usize,
    ) -> Result<Self, LoadEntriesError> {
        let mut set = Self {
            volume_label: None,
            allocation_bitmaps: [None, None],
        };

        'cluster_chain: for cluster_index in fat.get_cluster_chain(first_cluster) {
            // Create cluster reader.
            let mut reader = match ClusterReader::new(params, image, cluster_index) {
                Ok(v) => SetReader::new(v, cluster_index),
                Err(e) => {
                    return Err(LoadEntriesError::CreateClusterReaderFailed(
                        cluster_index,
                        e,
                    ));
                }
            };

            loop {
                // Read primary entry.
                let entry = reader.read()?;
                let ty = EntryType(entry.data[0]);

                if !ty.is_regular() {
                    break 'cluster_chain;
                } else if ty.type_category() != EntryType::PRIMARY {
                    return Err(LoadEntriesError::NotPrimary(entry.index, entry.cluster));
                }

                // Parse primary entry.
                match (ty.type_importance(), ty.type_code()) {
                    (EntryType::CRITICAL, 1) => set.read_allocation_bitmap(entry)?,
                    (EntryType::CRITICAL, 3) => set.read_volume_label(entry)?,
                    _ => {
                        return Err(LoadEntriesError::UnknownEntry(
                            ty,
                            entry.index,
                            entry.cluster,
                        ));
                    }
                }
            }
        }

        Ok(set)
    }

    fn read_allocation_bitmap(&mut self, entry: RawEntry) -> Result<(), LoadEntriesError> {
        // Get next index.
        let index = if self.allocation_bitmaps[1].is_some() {
            return Err(LoadEntriesError::TooManyAllocationBitmap);
        } else if self.allocation_bitmaps[0].is_some() {
            1
        } else {
            0
        };

        // Load fields.
        let data = entry.data.as_ptr();
        let bitmap_flags = read_u8(data, 1) as usize;

        if (bitmap_flags & 1) != index {
            return Err(LoadEntriesError::WrongAllocationBitmap);
        }

        // Update set.
        self.allocation_bitmaps[index] = Some(DataDescriptor::load(&entry)?);

        Ok(())
    }

    fn read_volume_label(&mut self, entry: RawEntry) -> Result<(), LoadEntriesError> {
        // Check if more than one volume label.
        if self.volume_label.is_some() {
            return Err(LoadEntriesError::MultipleVolumeLabel);
        }

        // Load fields.
        let data = entry.data;
        let character_count = data[1] as usize;

        if character_count > 11 {
            return Err(LoadEntriesError::InvalidVolumeLabel);
        }

        let volume_label = &data[2..(2 + character_count * 2)];

        // Update set.
        self.volume_label = Some(String::from_utf16_lossy(util::slice::from_bytes(
            volume_label,
        )));

        Ok(())
    }
}

struct SetReader<'a, I: Read + Seek> {
    cluster_reader: ClusterReader<'a, I>,
    cluster_index: usize,
    entry_index: usize,
}

impl<'a, I: Read + Seek> SetReader<'a, I> {
    fn new(cluster_reader: ClusterReader<'a, I>, cluster_index: usize) -> Self {
        Self {
            cluster_reader,
            cluster_index,
            entry_index: 0,
        }
    }

    fn read(&mut self) -> Result<RawEntry, LoadEntriesError> {
        let index = self.entry_index;
        let entry: [u8; 32] = match util::io::read_array(&mut self.cluster_reader) {
            Ok(v) => v,
            Err(e) => {
                return Err(LoadEntriesError::ReadEntryFailed(
                    index,
                    self.cluster_index,
                    e,
                ));
            }
        };

        self.entry_index += 1;

        Ok(RawEntry {
            index,
            cluster: self.cluster_index,
            data: entry,
        })
    }
}

struct RawEntry {
    index: usize,
    cluster: usize,
    data: [u8; 32],
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct EntryType(u8);

impl EntryType {
    pub const PRIMARY: u8 = 0;
    pub const SECONDARY: u8 = 1;
    pub const CRITICAL: u8 = 0;
    pub const BENIGN: u8 = 1;

    pub fn is_regular(self) -> bool {
        self.0 >= 0x81
    }

    pub fn type_code(self) -> u8 {
        self.0 & 0x1f
    }

    pub fn type_importance(self) -> u8 {
        (self.0 & 0x20) >> 5
    }

    pub fn type_category(self) -> u8 {
        (self.0 & 0x40) >> 6
    }
}

impl Display for EntryType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_regular() {
            if self.type_importance() == EntryType::CRITICAL {
                f.write_str("critical ")?;
            } else {
                f.write_str("benign ")?;
            }

            if self.type_category() == EntryType::PRIMARY {
                f.write_str("primary ")?;
            } else {
                f.write_str("secondary ")?;
            }

            write!(f, "{}", self.type_code())
        } else {
            write!(f, "{:#04x}", self.0)
        }
    }
}

pub(crate) struct DataDescriptor {
    first_cluster: usize,
    data_length: u64,
}

impl DataDescriptor {
    fn load(entry: &RawEntry) -> Result<Self, LoadEntriesError> {
        let data = entry.data.as_ptr();
        let first_cluster = read_u32_le(data, 20) as usize;
        let data_length = read_u64_le(data, 24);

        if first_cluster == 0 {
            if data_length != 0 {
                return Err(LoadEntriesError::InvalidDataLength(
                    entry.index,
                    entry.cluster,
                ));
            }
        } else if first_cluster < 2 {
            return Err(LoadEntriesError::InvalidFirstCluster(
                entry.index,
                entry.cluster,
            ));
        }

        Ok(Self {
            first_cluster,
            data_length,
        })
    }

    pub fn first_cluster(&self) -> usize {
        self.first_cluster
    }

    pub fn data_length(&self) -> u64 {
        self.data_length
    }
}

#[derive(Debug)]
pub enum LoadEntriesError {
    CreateClusterReaderFailed(usize, crate::cluster::NewError),
    ReadEntryFailed(usize, usize, std::io::Error),
    NotPrimary(usize, usize),
    UnknownEntry(EntryType, usize, usize),
    InvalidFirstCluster(usize, usize),
    InvalidDataLength(usize, usize),
    TooManyAllocationBitmap,
    WrongAllocationBitmap,
    MultipleVolumeLabel,
    InvalidVolumeLabel,
}

impl Error for LoadEntriesError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateClusterReaderFailed(_, e) => Some(e),
            Self::ReadEntryFailed(_, _, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadEntriesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateClusterReaderFailed(i, _) => {
                write!(f, "cannot create reader for cluster #{}", i)
            }
            Self::ReadEntryFailed(e, c, _) => {
                write!(f, "cannot read entry #{} from cluster #{}", e, c)
            }
            Self::NotPrimary(e, c) => {
                write!(f, "entry #{} from cluster #{} is not a primary entry", e, c)
            }
            Self::UnknownEntry(t, e, c) => {
                write!(f, "unknown entry #{} on cluster #{} ({})", e, c, t)
            }
            Self::InvalidFirstCluster(e, c) => write!(
                f,
                "invalid FirstCluster at entry #{} from cluster #{}",
                e, c
            ),
            Self::InvalidDataLength(e, c) => {
                write!(f, "invalid DataLength at entry #{} from cluster #{}", e, c)
            }
            Self::TooManyAllocationBitmap => {
                f.write_str("more than 2 allocation bitmaps exists in the directory")
            }
            Self::WrongAllocationBitmap => {
                f.write_str("allocation bitmap in the directory is not for its corresponding FAT")
            }
            Self::MultipleVolumeLabel => {
                f.write_str("multiple volume label exists in the directory")
            }
            Self::InvalidVolumeLabel => f.write_str("invalid volume label"),
        }
    }
}
