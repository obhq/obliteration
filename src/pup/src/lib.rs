use self::entry::Entry;
use self::reader::{BlockedReader, EntryReader, NonBlockedReader};
use exfat::ExFat;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{create_dir, File};
use std::io::{Read, Seek};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use thiserror::Error;
use util::mem::{read_array, read_u16_le};

pub mod entry;
pub mod reader;

#[no_mangle]
pub unsafe extern "C" fn pup_open(file: *const c_char, err: *mut *mut error::Error) -> *mut Pup {
    let file = unsafe { util::str::from_c_unchecked(file) };
    let pup = match Pup::open(file) {
        Ok(v) => Box::new(v),
        Err(e) => {
            unsafe { *err = error::Error::new(&e) };
            return null_mut();
        }
    };

    Box::into_raw(pup)
}

#[no_mangle]
pub unsafe extern "C" fn pup_dump_system(pup: &Pup, path: *const c_char) -> *mut error::Error {
    let path = unsafe { util::str::from_c_unchecked(path) };

    if let Err(e) = pup.dump_system_image(path) {
        return error::Error::new(&e);
    }

    null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn pup_free(pup: *mut Pup) {
    unsafe { Box::from_raw(pup) };
}

pub struct Pup {
    file: memmap2::Mmap,
    entries: Vec<Entry>,
}

impl Pup {
    pub fn open<F: AsRef<Path>>(file: F) -> Result<Self, OpenError> {
        // Open file and map it to memory.
        let file = match File::open(file) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::OpenFailed(e)),
        };

        let file = match unsafe { memmap2::Mmap::map(&file) } {
            Ok(v) => v,
            Err(e) => return Err(OpenError::MapFailed(e)),
        };

        if file.len() < 32 {
            return Err(OpenError::TooSmall);
        }

        // Check magic.
        let hdr = file.as_ptr();
        let magic: [u8; 4] = unsafe { read_array(hdr, 0) };

        if magic != [0x4f, 0x15, 0x3d, 0x1d] {
            return Err(OpenError::InvalidMagic);
        }

        // Read entry headers.
        let entry_count = unsafe { read_u16_le(hdr, 24) } as usize;
        let mut entries: Vec<Entry> = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let offset = 32 + i * Entry::RAW_SIZE;
            let entry = match file.get(offset..(offset + Entry::RAW_SIZE)) {
                Some(v) => Entry::read(v.as_ptr()),
                None => return Err(OpenError::TooSmall),
            };

            entries.push(entry);
        }

        Ok(Self { file, entries })
    }

    pub fn dump_system_image<O: AsRef<Path>>(&self, output: O) -> Result<(), DumpSystemImageError> {
        // Get entry.
        let (entry, index) = match self.get_data_entry(6) {
            Some(v) => v,
            None => return Err(DumpSystemImageError::EntryNotFound),
        };

        // Create entry reader.
        let entry = match self.create_reader(entry, index) {
            Ok(v) => v,
            Err(e) => return Err(DumpSystemImageError::CreateEntryReaderFailed(e)),
        };

        // Create exFAT reader.
        let fat = match ExFat::open(entry) {
            Ok(v) => v,
            Err(e) => return Err(DumpSystemImageError::CreateImageReaderFailed(e)),
        };

        // Dump files.
        let output = output.as_ref();

        for item in fat {
            use exfat::directory::Item;

            match item {
                Item::Directory(i) => Self::dump_system_dir(output, i)?,
                Item::File(i) => Self::dump_system_file(output, i)?,
            }
        }

        Ok(())
    }

    fn dump_system_dir<P, I>(
        parent: P,
        dir: exfat::directory::Directory<I>,
    ) -> Result<(), DumpSystemImageError>
    where
        P: AsRef<Path>,
        I: Read + Seek,
    {
        // Create a directory.
        let path = parent.as_ref().join(dir.name());

        if let Err(e) = create_dir(&path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(DumpSystemImageError::CreateDirectoryFailed(path, e));
            }
        }

        // Open the exFAT directory.
        let items = match dir.open() {
            Ok(v) => v,
            Err(e) => {
                return Err(DumpSystemImageError::OpenDirectoryFailed(
                    dir.name().into(),
                    e,
                ));
            }
        };

        // Dump files.
        for item in items {
            use exfat::directory::Item;

            match item {
                Item::Directory(i) => Self::dump_system_dir(&path, i)?,
                Item::File(i) => Self::dump_system_file(&path, i)?,
            }
        }

        Ok(())
    }

    fn dump_system_file<P, I>(_: P, _: exfat::file::File<I>) -> Result<(), DumpSystemImageError>
    where
        P: AsRef<Path>,
        I: Read + Seek,
    {
        // TODO: Write file.
        Ok(())
    }

    fn get_data_entry(&self, id: u16) -> Option<(&Entry, usize)> {
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];

            if entry.is_table() {
                continue;
            }

            if entry.id() == id {
                return Some((entry, i));
            }
        }

        None
    }

    fn create_reader<'a>(
        &'a self,
        entry: &'a Entry,
        index: usize,
    ) -> Result<Box<dyn EntryReader + 'a>, Box<dyn Error>> {
        let reader: Box<dyn EntryReader + 'a> = if entry.is_blocked() && entry.is_compressed() {
            let table = self
                .entries
                .iter()
                .position(|e| e.is_table() && e.id() as usize == index)
                .unwrap();
            let table = &self.entries[table];

            Box::new(BlockedReader::new(entry, table, &self.file)?)
        } else {
            Box::new(NonBlockedReader::new(entry, &self.file)?)
        };

        Ok(reader)
    }
}

#[derive(Debug)]
pub enum OpenError {
    OpenFailed(std::io::Error),
    MapFailed(std::io::Error),
    TooSmall,
    InvalidMagic,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenFailed(e) | Self::MapFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::OpenFailed(_) => f.write_str("cannot open file"),
            Self::MapFailed(_) => f.write_str("cannot map file"),
            Self::TooSmall => f.write_str("file too small"),
            Self::InvalidMagic => f.write_str("invalid magic"),
        }
    }
}

/// Represents an error for [`dump_system_image()`][Pup::dump_system_image()].
#[derive(Debug, Error)]
pub enum DumpSystemImageError {
    #[error("entry not found")]
    EntryNotFound,

    #[error("cannot create entry reader")]
    CreateEntryReaderFailed(#[source] Box<dyn Error>),

    #[error("cannot create exFAT reader for system image")]
    CreateImageReaderFailed(#[source] exfat::OpenError),

    #[error("cannot create directory {0}")]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot open directory {0} on the image")]
    OpenDirectoryFailed(String, #[source] exfat::directory::OpenError),
}
