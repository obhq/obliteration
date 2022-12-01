use crate::cluster::ClustersReader;
use crate::fat::Fat;
use crate::param::Params;
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};
use util::mem::{read_u16_le, read_u32_le, read_u64_le, read_u8};

pub(crate) struct EntrySet {
    pub allocation_bitmaps: [Option<DataDescriptor>; 2],
    pub upcase_table: Option<UpcaseTableDescriptor>,
    pub volume_label: Option<String>,
    pub files: Vec<FileDescriptor>,
}

impl EntrySet {
    pub fn load<I: Read + Seek>(
        params: &Params,
        fat: &Fat,
        image: &mut I,
        first_cluster: usize,
    ) -> Result<Self, LoadEntriesError> {
        let mut set = Self {
            allocation_bitmaps: [None, None],
            upcase_table: None,
            volume_label: None,
            files: Vec::new(),
        };

        let mut reader = match ClustersReader::new(params, fat, image, first_cluster, None) {
            Ok(v) => SetReader::new(v),
            Err(e) => return Err(LoadEntriesError::CreateClustersReaderFailed(e)),
        };

        loop {
            // Read primary entry.
            let entry = reader.read()?;
            let ty = EntryType(entry.data[0]);

            if !ty.is_regular() {
                break;
            } else if ty.type_category() != EntryType::PRIMARY {
                return Err(LoadEntriesError::NotPrimary(entry.index, entry.cluster));
            }

            // Parse primary entry.
            match (ty.type_importance(), ty.type_code()) {
                (EntryType::CRITICAL, 1) => set.read_allocation_bitmap(entry)?,
                (EntryType::CRITICAL, 2) => set.read_upcase_table(entry)?,
                (EntryType::CRITICAL, 3) => set.read_volume_label(entry)?,
                (EntryType::CRITICAL, 5) => set.read_file(entry, &mut reader)?,
                _ => {
                    return Err(LoadEntriesError::UnknownEntry(
                        ty,
                        entry.index,
                        entry.cluster,
                    ));
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

    fn read_upcase_table(&mut self, entry: RawEntry) -> Result<(), LoadEntriesError> {
        // Check if more than one up-case table.
        if self.upcase_table.is_some() {
            return Err(LoadEntriesError::MultipleUpcaseTable);
        }

        // Load fields.
        let data = entry.data.as_ptr();
        let checksum = read_u32_le(data, 4);
        let data = DataDescriptor::load(&entry)?;

        // Update set.
        self.upcase_table = Some(UpcaseTableDescriptor { checksum, data });

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

    fn read_file<I: Read + Seek>(
        &mut self,
        entry: RawEntry,
        directory: &mut SetReader<I>,
    ) -> Result<(), LoadEntriesError> {
        // Get number of secondary entries.
        let secondary_count = entry.data[1] as usize;

        if secondary_count < 1 {
            return Err(LoadEntriesError::NoStreamExtension(
                entry.index,
                entry.cluster,
            ));
        } else if secondary_count < 2 {
            return Err(LoadEntriesError::NoFileName(entry.index, entry.cluster));
        }

        // Read stream extension.
        let stream = directory.read()?;
        let ty = stream.ty();

        if !ty.is_critical_secondary(0) {
            return Err(LoadEntriesError::WrongEntry(
                ty,
                stream.index,
                stream.cluster,
            ));
        }

        // Read file names.
        let mut names: Vec<RawEntry> = Vec::with_capacity(secondary_count - 1);

        for _ in 0..names.capacity() {
            let entry = directory.read()?;
            let ty = entry.ty();

            if !ty.is_critical_secondary(1) {
                return Err(LoadEntriesError::WrongEntry(ty, entry.index, entry.cluster));
            }

            names.push(entry);
        }

        // Load fields.
        let data = entry.data.as_ptr();
        let file_attributes = FileAttributes(read_u16_le(data, 4));

        // Update set.
        let stream = Self::read_stream(stream)?;
        let name = Self::read_file_names(&entry, &stream, &names)?;

        self.files.push(FileDescriptor {
            attributes: file_attributes,
            stream,
            name,
        });

        Ok(())
    }

    fn read_stream(entry: RawEntry) -> Result<StreamDescriptor, LoadEntriesError> {
        // Load fields.
        let data = entry.data.as_ptr();
        let general_secondary_flags = SecondaryFlags(read_u8(data, 1));

        if !general_secondary_flags.allocation_possible() {
            return Err(LoadEntriesError::InvalidStreamExtension(
                entry.index,
                entry.cluster,
            ));
        }

        let name_length = read_u8(data, 3) as usize;

        if name_length < 1 {
            return Err(LoadEntriesError::InvalidStreamExtension(
                entry.index,
                entry.cluster,
            ));
        }

        let valid_data_length = read_u64_le(data, 8);
        let data = DataDescriptor::load(&entry)?;

        if valid_data_length > data.data_length {
            return Err(LoadEntriesError::InvalidStreamExtension(
                entry.index,
                entry.cluster,
            ));
        }

        Ok(StreamDescriptor {
            no_fat_chain: general_secondary_flags.no_fat_chain(),
            name_length,
            valid_data_length,
        })
    }

    fn read_file_names(
        file: &RawEntry,
        stream: &StreamDescriptor,
        names: &[RawEntry],
    ) -> Result<String, LoadEntriesError> {
        // TODO: Use div_ceil when https://github.com/rust-lang/rust/issues/88581 stabilized.
        if names.len() != (stream.name_length + 15 - 1) / 15 {
            return Err(LoadEntriesError::WrongFileNames(file.index, file.cluster));
        }

        let mut need = stream.name_length * 2;
        let mut name = String::with_capacity(15 * names.len());

        for entry in names {
            let data = &entry.data;
            let general_secondary_flags = SecondaryFlags(data[1]);

            if general_secondary_flags.allocation_possible() {
                return Err(LoadEntriesError::InvalidFileName(
                    entry.index,
                    entry.cluster,
                ));
            }

            let file_name = &data[2..(2 + min(30, need))];

            need -= file_name.len();

            match String::from_utf16(util::slice::from_bytes(file_name)) {
                Ok(v) => name.push_str(&v),
                Err(_) => {
                    return Err(LoadEntriesError::InvalidFileName(
                        entry.index,
                        entry.cluster,
                    ));
                }
            }
        }

        Ok(name)
    }
}

struct SetReader<'a, I: Read + Seek> {
    reader: ClustersReader<'a, I>,
    entry_index: usize,
}

impl<'a, I: Read + Seek> SetReader<'a, I> {
    fn new(reader: ClustersReader<'a, I>) -> Self {
        Self {
            reader,
            entry_index: 0,
        }
    }

    fn cluster_index(&self) -> usize {
        self.reader.cluster()
    }

    fn read(&mut self) -> Result<RawEntry, LoadEntriesError> {
        let cluster = self.cluster_index();
        let index = self.entry_index;
        let entry: [u8; 32] = match util::io::read_array(&mut self.reader) {
            Ok(v) => v,
            Err(e) => return Err(LoadEntriesError::ReadEntryFailed(index, cluster, e)),
        };

        if self.cluster_index() != cluster {
            self.entry_index = 0;
        } else {
            self.entry_index += 1;
        }

        Ok(RawEntry {
            index,
            cluster,
            data: entry,
        })
    }
}

struct RawEntry {
    index: usize,
    cluster: usize,
    data: [u8; 32],
}

impl RawEntry {
    fn ty(&self) -> EntryType {
        EntryType(self.data[0])
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

    pub fn is_critical_secondary(self, code: u8) -> bool {
        self.is_regular()
            && self.type_importance() == Self::CRITICAL
            && self.type_category() == Self::SECONDARY
            && self.type_code() == code
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

#[derive(Clone, Copy)]
#[repr(transparent)]
struct SecondaryFlags(u8);

impl SecondaryFlags {
    fn allocation_possible(self) -> bool {
        (self.0 & 1) != 0
    }

    fn no_fat_chain(self) -> bool {
        (self.0 & 2) != 0
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

pub(crate) struct UpcaseTableDescriptor {
    checksum: u32,
    data: DataDescriptor,
}

impl UpcaseTableDescriptor {
    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn data(&self) -> &DataDescriptor {
        &self.data
    }
}

pub(crate) struct FileDescriptor {
    attributes: FileAttributes,
    stream: StreamDescriptor,
    name: String,
}

impl FileDescriptor {
    pub fn attributes(&self) -> FileAttributes {
        self.attributes
    }

    pub fn stream(&self) -> &StreamDescriptor {
        &self.stream
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

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

pub(crate) struct StreamDescriptor {
    no_fat_chain: bool,
    name_length: usize,
    valid_data_length: u64,
}

impl StreamDescriptor {
    pub fn no_fat_chain(&self) -> bool {
        self.no_fat_chain
    }

    pub fn name_length(&self) -> usize {
        self.name_length
    }

    pub fn valid_data_length(&self) -> u64 {
        self.valid_data_length
    }
}

#[derive(Debug)]
pub enum LoadEntriesError {
    CreateClustersReaderFailed(crate::cluster::NewError),
    ReadEntryFailed(usize, usize, std::io::Error),
    NotPrimary(usize, usize),
    UnknownEntry(EntryType, usize, usize),
    WrongEntry(EntryType, usize, usize),
    InvalidFirstCluster(usize, usize),
    InvalidDataLength(usize, usize),
    TooManyAllocationBitmap,
    WrongAllocationBitmap,
    MultipleUpcaseTable,
    MultipleVolumeLabel,
    InvalidVolumeLabel,
    NoStreamExtension(usize, usize),
    NoFileName(usize, usize),
    InvalidStreamExtension(usize, usize),
    WrongFileNames(usize, usize),
    InvalidFileName(usize, usize),
}

impl Error for LoadEntriesError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateClustersReaderFailed(e) => Some(e),
            Self::ReadEntryFailed(_, _, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadEntriesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateClustersReaderFailed(_) => {
                f.write_str("cannot create a reader for cluster chain")
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
            Self::WrongEntry(t, e, c) => {
                write!(f, "entry #{} on cluster #{} cannot be {}", e, c, t)
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
            Self::MultipleUpcaseTable => {
                f.write_str("multiple up-case table exists in the directory")
            }
            Self::MultipleVolumeLabel => {
                f.write_str("multiple volume label exists in the directory")
            }
            Self::InvalidVolumeLabel => f.write_str("invalid volume label"),
            Self::NoStreamExtension(e, c) => write!(
                f,
                "no stream extension is followed entry #{} on cluster #{}",
                e, c
            ),
            Self::NoFileName(e, c) => {
                write!(f, "no file name is followed entry #{} on cluster #{}", e, c)
            }
            Self::InvalidStreamExtension(e, c) => write!(
                f,
                "entry #{} on cluster #{} is not a valid stream extension",
                e, c
            ),
            Self::WrongFileNames(c, e) => write!(
                f,
                "entry #{} on cluster #{} has wrong number of file names",
                e, c
            ),
            Self::InvalidFileName(c, e) => {
                write!(f, "entry #{} on cluster #{} is not a valid file name", e, c)
            }
        }
    }
}
