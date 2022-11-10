use crate::cluster::ClusterReader;
use crate::fat::Fat;
use crate::param::Params;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};

pub(crate) struct EntrySet {
    pub volume_label: Option<String>,
}

impl EntrySet {
    pub fn load<I: Read + Seek>(
        params: &Params,
        fat: &Fat,
        image: &mut I,
        first_cluster: usize,
    ) -> Result<Self, LoadEntriesError> {
        let mut set = Self { volume_label: None };

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
                let (entry_index, entry) = reader.read()?;
                let ty = EntryType(entry[0]);

                if !ty.is_regular() {
                    break 'cluster_chain;
                } else if ty.type_category() != EntryType::PRIMARY {
                    return Err(LoadEntriesError::NotPrimary(entry_index, cluster_index));
                }

                // Parse primary entry.
                match (ty.type_importance(), ty.type_code()) {
                    (EntryType::CRITICAL, 3) => set.read_volume_label(entry)?,
                    _ => {
                        return Err(LoadEntriesError::UnknownEntry(
                            ty,
                            entry_index,
                            cluster_index,
                        ));
                    }
                }
            }
        }

        Ok(set)
    }

    fn read_volume_label(&mut self, entry: [u8; 32]) -> Result<(), LoadEntriesError> {
        // Check if more than one volume label.
        if self.volume_label.is_some() {
            return Err(LoadEntriesError::MultipleVolumeLabel);
        }

        // Load fields.
        let character_count = entry[1] as usize;

        if character_count > 11 {
            return Err(LoadEntriesError::InvalidVolumeLabel);
        }

        let volume_label = &entry[2..(2 + character_count * 2)];

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

    fn read(&mut self) -> Result<(usize, [u8; 32]), LoadEntriesError> {
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

        Ok((index, entry))
    }
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

#[derive(Debug)]
pub enum LoadEntriesError {
    CreateClusterReaderFailed(usize, crate::cluster::NewError),
    ReadEntryFailed(usize, usize, std::io::Error),
    NotPrimary(usize, usize),
    UnknownEntry(EntryType, usize, usize),
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
            Self::MultipleVolumeLabel => f.write_str("multiple volume label"),
            Self::InvalidVolumeLabel => f.write_str("invalid volume label"),
        }
    }
}
