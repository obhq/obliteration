use self::entry::Entry;
use self::reader::{BlockedReader, EntryReader, NonBlockedReader};
use exfat::ExFat;
use std::error::Error;
use std::ffi::{c_void, CString};
use std::fmt::{Display, Formatter};
use std::fs::{create_dir, File};
use std::io::{Read, Seek, Write};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use thiserror::Error;
use util::mem::{new_buffer, read_array, read_u16_le};

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
pub unsafe extern "C" fn pup_dump_system(
    pup: &Pup,
    path: *const c_char,
    status: extern "C" fn(*const c_char, u64, u64, *mut c_void),
    ud: *mut c_void,
) -> *mut error::Error {
    let path = unsafe { util::str::from_c_unchecked(path) };

    if let Err(e) = pup.dump_system_image(path, status, ud) {
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

    pub fn dump_system_image<O: AsRef<Path>>(
        &self,
        output: O,
        status: extern "C" fn(*const c_char, u64, u64, *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), DumpSystemImageError> {
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
                Item::Directory(i) => Self::dump_system_dir(output, i, status, ud)?,
                Item::File(i) => Self::dump_system_file(output, i, status, ud)?,
            }
        }

        Ok(())
    }

    fn dump_system_dir<P, I>(
        parent: P,
        dir: exfat::directory::Directory<I>,
        status: extern "C" fn(*const c_char, u64, u64, *mut c_void),
        ud: *mut c_void,
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
            Err(e) => return Err(DumpSystemImageError::OpenDirectoryFailed(path, e)),
        };

        // Dump files.
        for item in items {
            use exfat::directory::Item;

            match item {
                Item::Directory(i) => Self::dump_system_dir(&path, i, status, ud)?,
                Item::File(i) => Self::dump_system_file(&path, i, status, ud)?,
            }
        }

        Ok(())
    }

    fn dump_system_file<P, I>(
        parent: P,
        mut file: exfat::file::File<I>,
        status: extern "C" fn(*const c_char, u64, u64, *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), DumpSystemImageError>
    where
        P: AsRef<Path>,
        I: Read + Seek,
    {
        let path = parent.as_ref().join(file.name());
        let len = file.len();

        // Open the exFAT file.
        let reader = match file.open() {
            Ok(v) => v,
            Err(e) => return Err(DumpSystemImageError::OpenFileFailed(path, e)),
        };

        // Create a destination file.
        let mut writer = match File::create(&path) {
            Ok(v) => v,
            Err(e) => return Err(DumpSystemImageError::CreateFileFailed(path, e)),
        };

        // Check if an empty file.
        let mut reader = match reader {
            Some(v) => v,
            None => return Ok(()),
        };

        // Report initial status.
        let display = CString::new(path.to_string_lossy().as_ref()).unwrap();

        status(display.as_ptr(), len, 0, ud);

        // Copy content.
        let mut buf = unsafe { new_buffer(32768) };
        let mut written = 0;

        loop {
            // Read the source.
            let read = match reader.read(&mut buf) {
                Ok(v) => v,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    } else {
                        return Err(DumpSystemImageError::ReadFileFailed(path, e));
                    }
                }
            };

            if read == 0 {
                break;
            }

            // Write destination.
            if let Err(e) = writer.write_all(&mut buf[..read]) {
                return Err(DumpSystemImageError::WriteFileFailed(path, e));
            }

            written += read;

            // Report status.
            status(display.as_ptr(), len, written as u64, ud);
        }

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

    #[error("cannot open a corresponding directory {0} on the image")]
    OpenDirectoryFailed(PathBuf, #[source] exfat::directory::OpenError),

    #[error("cannot open a corresponding file {0} on the image")]
    OpenFileFailed(PathBuf, #[source] exfat::file::OpenError),

    #[error("cannot create file {0}")]
    CreateFileFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot read a corresponding file {0} on the image")]
    ReadFileFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot write {0}")]
    WriteFileFailed(PathBuf, #[source] std::io::Error),
}
