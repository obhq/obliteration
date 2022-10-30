use self::entry::{BlockedReader, ContiguousReader, Entry};
use exfat::ExFat;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;
use std::ptr::null_mut;
use util::mem::{read_array, read_u16_le};

pub mod entry;

#[no_mangle]
pub extern "C" fn pup_open(file: *const c_char, err: *mut *mut error::Error) -> *mut Pup {
    let file = util::str::from_c_unchecked(file);
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
pub extern "C" fn pup_dump_system(pup: &Pup, path: *const c_char) -> *mut error::Error {
    let path = util::str::from_c_unchecked(path);

    if let Err(e) = pup.dump_system_image(path) {
        return error::Error::new(&e);
    }

    null_mut()
}

#[no_mangle]
pub extern "C" fn pup_free(pup: *mut Pup) {
    unsafe { Box::from_raw(pup) };
}

pub struct Pup {
    file: memmap2::Mmap,
    entries: Vec<Entry>,
    table_entries: Vec<Option<usize>>,
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
        let magic: [u8; 4] = read_array(hdr, 0);

        if magic != [0x4f, 0x15, 0x3d, 0x1d] {
            return Err(OpenError::InvalidMagic);
        }

        // Read entry headers.
        let entry_count = read_u16_le(hdr, 24) as usize;
        let mut entries: Vec<Entry> = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let offset = 32 + i * Entry::RAW_SIZE;
            let entry = match file.get(offset..(offset + Entry::RAW_SIZE)) {
                Some(v) => Entry::read(v.as_ptr()),
                None => return Err(OpenError::TooSmall),
            };

            entries.push(entry);
        }

        // TODO: What is table?
        let mut table_entries: Vec<Option<usize>> = vec![None; entries.len()];

        for i in 0..entries.len() {
            let entry = &entries[i];

            if entry.is_blocked() {
                if ((entry.id() | 0x100) & 0xf00) == 0xf00 {
                    // What is this?
                    todo!();
                }

                let table = entries
                    .iter()
                    .position(|e| (e.flags() & 1) != 0 && (e.id() as usize) == i)
                    .unwrap();

                if table_entries[table].is_some() {
                    // What is this?
                    todo!();
                }

                table_entries[table] = Some(i);
            } else {
                table_entries[i] = None;
            }
        }

        Ok(Self {
            file,
            entries,
            table_entries,
        })
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
        let fat = match ExFat::open(entry, false) {
            Ok(v) => v,
            Err(e) => return Err(DumpSystemImageError::CreateImageReaderFailed(e)),
        };

        Ok(())
    }

    fn get_data_entry(&self, id: u16) -> Option<(&Entry, usize)> {
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let special = entry.flags() & 0xf0000000;

            if special == 0xe0000000 || special == 0xf0000000 || self.table_entries[i].is_some() {
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
    ) -> Result<Box<dyn Read + 'a>, entry::ReaderError> {
        let reader: Box<dyn Read + 'a> = if entry.is_blocked() {
            let table = self
                .entries
                .iter()
                .position(|e| (e.flags() & 1) != 0 && e.id() as usize == index)
                .unwrap();

            Box::new(BlockedReader::new(entry, &self.entries[table], &self.file)?)
        } else {
            Box::new(ContiguousReader::new(entry, &self.file))
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

#[derive(Debug)]
pub enum DumpSystemImageError {
    EntryNotFound,
    CreateEntryReaderFailed(entry::ReaderError),
    CreateImageReaderFailed(exfat::OpenError),
}

impl Error for DumpSystemImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateEntryReaderFailed(e) => Some(e),
            Self::CreateImageReaderFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for DumpSystemImageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntryNotFound => f.write_str("entry not found"),
            Self::CreateEntryReaderFailed(_) => f.write_str("cannot create entry reader"),
            Self::CreateImageReaderFailed(_) => {
                f.write_str("cannot create reader for system image")
            }
        }
    }
}
