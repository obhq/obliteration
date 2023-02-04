use self::entry::Entry;
use self::header::Header;
use self::keys::{fake_pfs_key, pkg_key3};
use self::param::Param;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use sha2::Digest;
use std::error::Error;
use std::ffi::{c_void, CString};
use std::fmt::{Display, Formatter};
use std::fs::{create_dir_all, File};
use std::io::{Cursor, Read, Write};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use thiserror::Error;
use util::mem::{new_buffer, uninit};

pub mod entry;
pub mod header;
pub mod keys;
pub mod param;

#[no_mangle]
pub extern "C" fn pkg_open(file: *const c_char, error: *mut *mut error::Error) -> *mut Pkg {
    let path = util::str::from_c_unchecked(file);
    let pkg = match Pkg::open(path) {
        Ok(v) => Box::new(v),
        Err(e) => {
            unsafe { *error = error::Error::new(&e) };
            return null_mut();
        }
    };

    Box::into_raw(pkg)
}

#[no_mangle]
pub extern "C" fn pkg_close(pkg: *mut Pkg) {
    unsafe { Box::from_raw(pkg) };
}

#[no_mangle]
pub extern "C" fn pkg_get_param(pkg: &Pkg, error: *mut *mut c_char) -> *mut Param {
    let param = match pkg.get_param() {
        Ok(v) => Box::new(v),
        Err(e) => {
            util::str::set_c(error, &e.to_string());
            return null_mut();
        }
    };

    Box::into_raw(param)
}

#[no_mangle]
pub extern "C" fn pkg_extract(
    pkg: &Pkg,
    dir: *const c_char,
    status: extern "C" fn(*const c_char, u64, u64, ud: *mut c_void),
    ud: *mut c_void,
) -> *mut error::Error {
    let dir = util::str::from_c_unchecked(dir);

    match pkg.extract(dir, status, ud) {
        Ok(_) => null_mut(),
        Err(e) => error::Error::new(&e),
    }
}

#[no_mangle]
pub extern "C" fn pkg_param_open(file: *const c_char, error: *mut *mut c_char) -> *mut Param {
    // Open file.
    let mut file = match File::open(util::str::from_c_unchecked(file)) {
        Ok(v) => v,
        Err(e) => {
            util::str::set_c(error, &e.to_string());
            return null_mut();
        }
    };

    // param.sfo is quite small so we can read all of it content into memory.
    let mut data: Vec<u8> = Vec::new();

    match file.metadata() {
        Ok(v) => {
            if v.len() <= 4096 {
                if let Err(e) = file.read_to_end(&mut data) {
                    util::str::set_c(error, &e.to_string());
                    return null_mut();
                }
            } else {
                util::str::set_c(error, "file too large");
                return null_mut();
            }
        }
        Err(e) => {
            util::str::set_c(error, &e.to_string());
            return null_mut();
        }
    };

    // Parse.
    let param = match Param::read(&data) {
        Ok(v) => Box::new(v),
        Err(e) => {
            util::str::set_c(error, &e.to_string());
            return null_mut();
        }
    };

    Box::into_raw(param)
}

#[no_mangle]
pub extern "C" fn pkg_param_title_id(param: &Param) -> *mut c_char {
    util::str::to_c(param.title_id())
}

#[no_mangle]
pub extern "C" fn pkg_param_title(param: &Param) -> *mut c_char {
    util::str::to_c(param.title())
}

#[no_mangle]
pub extern "C" fn pkg_param_close(param: *mut Param) {
    unsafe { Box::from_raw(param) };
}

// https://www.psdevwiki.com/ps4/Package_Files
pub struct Pkg {
    raw: memmap2::Mmap,
    header: Header,
    entry_key3: Vec<u8>,
    ekpfs: Vec<u8>,
}

impl Pkg {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OpenError> {
        // Open file and map it to memory.
        let file = match File::open(path) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::OpenFailed(e)),
        };

        let raw = match unsafe { memmap2::Mmap::map(&file) } {
            Ok(v) => v,
            Err(e) => return Err(OpenError::MapFailed(e)),
        };

        // Read header.
        let header = match Header::read(raw.as_ref()) {
            Ok(v) => v,
            Err(_) => return Err(OpenError::InvalidHeader),
        };

        // Populate fields.
        let mut pkg = Self {
            raw,
            header,
            entry_key3: Vec::new(),
            ekpfs: Vec::new(),
        };

        pkg.load_entry_key3()?;
        pkg.load_ekpfs()?;

        Ok(pkg)
    }

    pub fn get_param(&self) -> Result<Param, GetParamError> {
        // Find an entry for param.sfo.
        let (_, _, data) = match self.find_entry(Entry::PARAM_SFO) {
            Ok(v) => v,
            Err(e) => return Err(GetParamError::FindEntryFailed(e)),
        };

        // Parse data.
        let param = match Param::read(data) {
            Ok(v) => v,
            Err(e) => return Err(GetParamError::ReadFailed(e)),
        };

        Ok(param)
    }

    pub fn extract<D: AsRef<Path>>(
        &self,
        dir: D,
        status: extern "C" fn(*const c_char, u64, u64, ud: *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), ExtractError> {
        let dir = dir.as_ref();

        self.extract_entries(dir.join("sce_sys"), status, ud)?;
        self.extract_pfs(dir, status, ud)?;

        Ok(())
    }

    fn extract_entries<D: AsRef<Path>>(
        &self,
        dir: D,
        status: extern "C" fn(*const c_char, u64, u64, ud: *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), ExtractError> {
        for num in 0..self.header.entry_count() {
            // Check offset.
            let offset = self.header.table_offset() + num * Entry::RAW_SIZE;
            let raw = match self.raw.get(offset..(offset + Entry::RAW_SIZE)) {
                Some(v) => v.as_ptr(),
                None => return Err(ExtractError::InvalidEntryOffset(num)),
            };

            // Read entry.
            let entry = Entry::read(raw);

            // Get file path.
            let path = match entry.to_path(dir.as_ref()) {
                Some(v) => v,
                None => continue,
            };

            // Check if we have a decryption key.
            if entry.is_encrypted() && (entry.key_index() != 3 || self.entry_key3.is_empty()) {
                return Err(ExtractError::NoEntryDecryptionKey(num));
            }

            // Get entry data.
            let offset = entry.data_offset();
            let size = if entry.is_encrypted() {
                (entry.data_size() + 15) & !15 // We need to include padding.
            } else {
                entry.data_size()
            };

            let data = match self.raw.get(offset..(offset + size)) {
                Some(v) => v,
                None => return Err(ExtractError::InvalidEntryDataOffset(num)),
            };

            // Report status.
            let name = CString::new(path.to_string_lossy().as_ref()).unwrap();

            status(name.as_ptr(), size as u64, 0, ud);

            // Create a directory for destination file.
            let dir = path.parent().unwrap();

            if let Err(e) = create_dir_all(dir) {
                return Err(ExtractError::CreateDirectoryFailed(dir.to_path_buf(), e));
            }

            // Open destination file.
            let mut file = match File::create(&path) {
                Ok(v) => v,
                Err(e) => return Err(ExtractError::CreateEntryFailed(path, e)),
            };

            // Dump entry data.
            if entry.is_encrypted() {
                if let Err(e) = self.decrypt_entry_data(&entry, data, |b| file.write_all(&b)) {
                    return Err(ExtractError::WriteEntryFailed(path, e));
                }
            } else if let Err(e) = file.write_all(data) {
                return Err(ExtractError::WriteEntryFailed(path, e));
            }

            // Report status.
            status(name.as_ptr(), size as u64, size as u64, ud);
        }

        Ok(())
    }

    fn extract_pfs<D: AsRef<Path>>(
        &self,
        dir: D,
        status: extern "C" fn(*const c_char, u64, u64, ud: *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), ExtractError> {
        use pfs::directory::Item;

        // Get outer PFS.
        let image_offset = self.header.pfs_offset();
        let image_size = self.header.pfs_size();
        let outer = match self.raw.get(image_offset..(image_offset + image_size)) {
            Some(v) => v,
            None => return Err(ExtractError::InvalidOuterOffset),
        };

        // Open outer PFS.
        let mut outer = match pfs::open(Cursor::new(outer), Some(&self.ekpfs)) {
            Ok(v) => match v.open() {
                Ok(v) => v,
                Err(e) => return Err(ExtractError::OpenOuterSuperRootFailed(e)),
            },
            Err(e) => return Err(ExtractError::OpenOuterFailed(e)),
        };

        // Open outer uroot directory.
        let mut uroot = match outer.take(b"uroot") {
            Some(v) => match v {
                Item::Directory(v) => match v.open() {
                    Ok(v) => v,
                    Err(e) => return Err(ExtractError::OpenOuterUrootFailed(e)),
                },
                Item::File(_) => return Err(ExtractError::NoOuterUroot),
            },
            None => return Err(ExtractError::NoOuterUroot),
        };

        // Get inner PFS.
        let inner = match uroot.take(b"pfs_image.dat") {
            Some(v) => match v {
                Item::Directory(_) => return Err(ExtractError::NoInnerImage),
                Item::File(v) => v,
            },
            None => return Err(ExtractError::NoInnerImage),
        };

        // Open inner PFS.
        let mut inner = if inner.is_compressed() {
            let pfsc = match pfs::pfsc::Reader::open(inner) {
                Ok(v) => v,
                Err(e) => return Err(ExtractError::CreateInnerDecompressorFailed(e)),
            };

            match pfs::open(pfsc, None) {
                Ok(v) => match v.open() {
                    Ok(v) => v,
                    Err(e) => return Err(ExtractError::OpenInnerSuperRootFailed(e)),
                },
                Err(e) => return Err(ExtractError::OpenInnerFailed(e)),
            }
        } else {
            match pfs::open(inner, None) {
                Ok(v) => match v.open() {
                    Ok(v) => v,
                    Err(e) => return Err(ExtractError::OpenInnerSuperRootFailed(e)),
                },
                Err(e) => return Err(ExtractError::OpenInnerFailed(e)),
            }
        };

        // Open inner uroot directory.
        let uroot = match inner.take(b"uroot") {
            Some(v) => match v {
                Item::Directory(v) => v,
                Item::File(_) => return Err(ExtractError::NoInnerUroot),
            },
            None => return Err(ExtractError::NoInnerUroot),
        };

        // Extract inner uroot.
        self.extract_directory(Vec::new(), uroot, dir, status, ud)
    }

    fn extract_directory<O: AsRef<Path>>(
        &self,
        path: Vec<&[u8]>,
        dir: pfs::directory::Directory,
        output: O,
        status: extern "C" fn(*const c_char, u64, u64, ud: *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), ExtractError> {
        // Open PFS directory.
        let items = match dir.open() {
            Ok(v) => v,
            Err(e) => {
                return Err(ExtractError::OpenDirectoryFailed(
                    self.build_pfs_path(&path),
                    e,
                ));
            }
        };

        // Enumerate items.
        let mut buffer: Vec<u8> = new_buffer(32768);

        for (name, item) in items {
            use pfs::directory::Item;

            // Build source path.
            let mut path = path.clone();

            path.push(&name);

            // Build destination path.
            let mut output = output.as_ref().to_path_buf();
            let name = match std::str::from_utf8(&name) {
                Ok(v) => v,
                Err(_) => {
                    return Err(ExtractError::UnsupportedFileName(
                        self.build_pfs_path(&path),
                    ));
                }
            };

            output.push(name);

            // Extract item.
            let meta = match item {
                Item::Directory(i) => {
                    // Constructe metadata.
                    let meta = fs::Metadata {
                        mode: i.mode().into(),
                        atime: i.atime(),
                        mtime: i.mtime(),
                        ctime: i.ctime(),
                        birthtime: i.birthtime(),
                        mtimensec: i.mtimensec(),
                        atimensec: i.atimensec(),
                        ctimensec: i.ctimensec(),
                        birthnsec: i.birthnsec(),
                        uid: i.uid(),
                        gid: i.gid(),
                    };

                    // Create output directory.
                    if let Err(e) = create_dir_all(&output) {
                        return Err(ExtractError::CreateDirectoryFailed(output, e));
                    }

                    // Extract files.
                    self.extract_directory(path, i, &output, status, ud)?;

                    meta
                }
                Item::File(mut file) => {
                    // Construct metadata.
                    let meta = fs::Metadata {
                        mode: file.mode().into(),
                        atime: file.atime(),
                        mtime: file.mtime(),
                        ctime: file.ctime(),
                        birthtime: file.birthtime(),
                        mtimensec: file.mtimensec(),
                        atimensec: file.atimensec(),
                        ctimensec: file.ctimensec(),
                        birthnsec: file.birthnsec(),
                        uid: file.uid(),
                        gid: file.gid(),
                    };

                    // Report initial status.
                    let size = file.len();
                    let status_name = CString::new(name).unwrap();

                    status(status_name.as_ptr(), size, 0, ud);

                    // Open destination file.
                    let mut dest = std::fs::OpenOptions::new();

                    dest.create_new(true);
                    dest.write(true);

                    let mut dest = match dest.open(&output) {
                        Ok(v) => v,
                        Err(e) => return Err(ExtractError::CreateFileFailed(output, e)),
                    };

                    // Copy.
                    let mut written = 0u64;

                    loop {
                        // Read source.
                        let read = match file.read(&mut buffer) {
                            Ok(v) => v,
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::Interrupted {
                                    continue;
                                } else {
                                    return Err(ExtractError::ReadFileFailed(
                                        self.build_pfs_path(&path),
                                        e,
                                    ));
                                }
                            }
                        };

                        if read == 0 {
                            break;
                        }

                        // Write destination.
                        if let Err(e) = dest.write_all(&buffer[..read]) {
                            return Err(ExtractError::WriteFileFailed(output, e));
                        }

                        written += read as u64; // Buffer size just 32768.

                        // Update status.
                        status(status_name.as_ptr(), size, written, ud);
                    }

                    meta
                }
            };

            // Create metadata.
            if let Err(e) = meta.create_for(&output) {
                return Err(ExtractError::CreateMetadataFailed(output, e));
            }
        }

        Ok(())
    }

    /// Gets a full path represents the item in PFS suitable for display to the user.
    fn build_pfs_path(&self, path: &[&[u8]]) -> String {
        let mut r = String::new();

        r.push('/');

        for c in path {
            r.push_str(&String::from_utf8_lossy(c));
        }

        r
    }

    fn load_ekpfs(&mut self) -> Result<(), OpenError> {
        // Locate image key entry.
        let (entry, _, data) = match self.find_entry(Entry::PFS_IMAGE_KEY) {
            Ok(v) => v,
            Err(e) => match e {
                FindEntryError::NotFound => return Err(OpenError::PfsImageKeyNotFound),
                _ => return Err(OpenError::FindPfsImageKeyFailed(e)),
            },
        };

        // Decrypt the entry.
        let mut encrypted: Vec<u8> = Vec::with_capacity(data.len());

        let _ = self.decrypt_entry_data(&entry, data, |b| -> Result<(), ()> {
            encrypted.extend(b);
            Ok(())
        });

        // Decrypt EKPFS with fake pkg key.
        let fake_key = fake_pfs_key();

        self.ekpfs = match fake_key.decrypt(rsa::Pkcs1v15Encrypt, &encrypted) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::DecryptEkpsfFailed(e)),
        };

        Ok(())
    }

    fn decrypt_entry_data<O, E>(
        &self,
        entry: &Entry,
        mut encrypted: &[u8],
        mut output: O,
    ) -> Result<(), E>
    where
        O: FnMut([u8; 16]) -> Result<(), E>,
    {
        if encrypted.len() % 16 != 0 {
            panic!("The size of encrypted data must be multiply of 16");
        }

        // Setup decryptor.
        let (key, iv) = self.derive_entry_key3(entry);
        let mut decryptor = cbc::Decryptor::<aes::Aes128>::new(&key.into(), &iv.into());

        // Dump blocks.
        loop {
            let mut block: [u8; 16] = uninit();

            encrypted = match util::array::read_from_slice(&mut block, encrypted) {
                Some(v) => v,
                None => break,
            };

            decryptor.decrypt_block_mut(GenericArray::from_mut_slice(&mut block));

            let result = output(block);

            if result.is_err() {
                return result;
            }
        }

        Ok(())
    }

    /// Get key and IV for `entry` using `entry_key3`. The caller **MUST** check if `entry_key3` is
    /// not empty before calling this method.
    fn derive_entry_key3(&self, entry: &Entry) -> ([u8; 16], [u8; 16]) {
        // Get secret seed.
        let mut seed = Vec::from(entry.to_bytes());

        seed.extend(self.entry_key3.as_slice());

        // Calculate secret.
        let mut sha256 = sha2::Sha256::new();

        sha256.update(seed.as_slice());

        let secret = sha256.finalize();

        // Extract key and IV.
        let mut key: [u8; 16] = uninit();
        let mut iv: [u8; 16] = uninit();
        let mut p = secret.as_ptr();

        p = util::array::read_from_ptr(&mut iv, p);
        util::array::read_from_ptr(&mut key, p);

        (key, iv)
    }

    fn load_entry_key3(&mut self) -> Result<(), OpenError> {
        // Locate entry keys.
        let (_, index, mut data) = match self.find_entry(Entry::ENTRY_KEYS) {
            Ok(v) => v,
            Err(e) => match e {
                FindEntryError::NotFound => return Err(OpenError::EntryKeyNotFound),
                _ => return Err(OpenError::FindEntryKeyFailed(e)),
            },
        };

        // Read seed.
        let mut seed: [u8; 32] = uninit();

        data = match util::array::read_from_slice(&mut seed, data) {
            Some(v) => v,
            None => return Err(OpenError::InvalidEntryOffset(index)),
        };

        // Read digests.
        let mut digests: [[u8; 32]; 7] = uninit();

        for i in 0..7 {
            data = match util::array::read_from_slice(&mut digests[i], data) {
                Some(v) => v,
                None => return Err(OpenError::InvalidEntryOffset(index)),
            };
        }

        // Read keys.
        let mut keys: [[u8; 256]; 7] = uninit();

        for i in 0..7 {
            data = match util::array::read_from_slice(&mut keys[i], data) {
                Some(v) => v,
                None => return Err(OpenError::InvalidEntryOffset(index)),
            };
        }

        // Decrypt key 3.
        let key3 = pkg_key3();

        self.entry_key3 = match key3.decrypt(rsa::Pkcs1v15Encrypt, &keys[3]) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::DecryptEntryKeyFailed(3, e)),
        };

        Ok(())
    }

    fn find_entry<'a>(&'a self, id: u32) -> Result<(Entry, usize, &'a [u8]), FindEntryError> {
        for num in 0..self.header.entry_count() {
            // Check offset.
            let offset = self.header.table_offset() + num * Entry::RAW_SIZE;
            let raw = match self.raw.get(offset..(offset + Entry::RAW_SIZE)) {
                Some(v) => v.as_ptr(),
                None => return Err(FindEntryError::InvalidEntryOffset(num)),
            };

            // Read entry.
            let entry = Entry::read(raw);

            if entry.id() != id {
                continue;
            }

            // Get entry data.
            let offset = entry.data_offset();
            let size = if entry.is_encrypted() {
                (entry.data_size() + 15) & !15 // We need to include padding.
            } else {
                entry.data_size()
            };

            let data = match self.raw.get(offset..(offset + size)) {
                Some(v) => v,
                None => return Err(FindEntryError::InvalidDataOffset(num)),
            };

            return Ok((entry, num, data));
        }

        Err(FindEntryError::NotFound)
    }
}

#[derive(Debug)]
pub enum OpenError {
    OpenFailed(std::io::Error),
    MapFailed(std::io::Error),
    InvalidHeader,
    EntryKeyNotFound,
    FindEntryKeyFailed(FindEntryError),
    InvalidEntryOffset(usize),
    DecryptEntryKeyFailed(usize, rsa::errors::Error),
    PfsImageKeyNotFound,
    FindPfsImageKeyFailed(FindEntryError),
    DecryptEkpsfFailed(rsa::errors::Error),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenFailed(e) => Some(e),
            Self::MapFailed(e) => Some(e),
            Self::FindEntryKeyFailed(e) => Some(e),
            Self::DecryptEntryKeyFailed(_, e) => Some(e),
            Self::FindPfsImageKeyFailed(e) => Some(e),
            Self::DecryptEkpsfFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::OpenFailed(e) => e.fmt(f),
            Self::MapFailed(e) => e.fmt(f),
            Self::InvalidHeader => f.write_str("PKG header is not valid"),
            Self::EntryKeyNotFound => f.write_str("no PKG entry key available"),
            Self::FindEntryKeyFailed(e) => e.fmt(f),
            Self::InvalidEntryOffset(num) => write!(f, "entry #{} has invalid data offset", num),
            Self::DecryptEntryKeyFailed(k, _) => write!(f, "cannot decrypt entry key #{}", k),
            Self::PfsImageKeyNotFound => f.write_str("no PFS image key in the PKG"),
            Self::FindPfsImageKeyFailed(e) => e.fmt(f),
            Self::DecryptEkpsfFailed(_) => f.write_str("cannot decrypt EKPFS"),
        }
    }
}

#[derive(Debug)]
pub enum GetParamError {
    FindEntryFailed(FindEntryError),
    ReadFailed(param::ReadError),
}

impl Error for GetParamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FindEntryFailed(e) => Some(e),
            Self::ReadFailed(e) => Some(e),
        }
    }
}

impl Display for GetParamError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::FindEntryFailed(e) => match e {
                FindEntryError::NotFound => f.write_str("the PKG does not have param.sfo"),
                _ => e.fmt(f),
            },
            Self::ReadFailed(_) => f.write_str("the PKG has malformed param.sfo"),
        }
    }
}

/// Errors for [`extract()`][Pkg::extract()].
#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("cannot create a directory {0}")]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),

    #[error("entry #{0} has invalid offset")]
    InvalidEntryOffset(usize),

    #[error("no decryption key for entry #{0}")]
    NoEntryDecryptionKey(usize),

    #[error("entry #{0} has invalid data offset")]
    InvalidEntryDataOffset(usize),

    #[error("cannot create {0}")]
    CreateEntryFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot write {0}")]
    WriteEntryFailed(PathBuf, #[source] std::io::Error),

    #[error("invalid offset for outer PFS")]
    InvalidOuterOffset,

    #[error("cannot open outer PFS")]
    OpenOuterFailed(#[source] pfs::OpenError),

    #[error("cannot open a super-root on outer PFS")]
    OpenOuterSuperRootFailed(#[source] pfs::directory::OpenError),

    #[error("no uroot directory on outer PFS")]
    NoOuterUroot,

    #[error("cannot open a uroot directory on outer PFS")]
    OpenOuterUrootFailed(#[source] pfs::directory::OpenError),

    #[error("outer PFS does not contains pfs_image.dat")]
    NoInnerImage,

    #[error("cannot create a decompressor for inner PFS")]
    CreateInnerDecompressorFailed(#[source] pfs::pfsc::OpenError),

    #[error("cannot open inner PFS")]
    OpenInnerFailed(#[source] pfs::OpenError),

    #[error("cannot open a super-root on inner PFS")]
    OpenInnerSuperRootFailed(#[source] pfs::directory::OpenError),

    #[error("no uroot directory on inner PFS")]
    NoInnerUroot,

    #[error("cannot open directory {0} on inner PFS")]
    OpenDirectoryFailed(String, #[source] pfs::directory::OpenError),

    #[error("directory {0} on inner PFS has file(s) with unsupported name")]
    UnsupportedFileName(String),

    #[error("cannot create a file {0}")]
    CreateFileFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot read {0} on inner PFS")]
    ReadFileFailed(String, #[source] std::io::Error),

    #[error("cannot write {0}")]
    WriteFileFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot create metadata for {0}")]
    CreateMetadataFailed(PathBuf, #[source] fs::CreateForError),
}

#[derive(Debug)]
pub enum FindEntryError {
    InvalidEntryOffset(usize),
    NotFound,
    InvalidDataOffset(usize),
}

impl Error for FindEntryError {}

impl Display for FindEntryError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidEntryOffset(num) => write!(f, "entry #{} has invalid offset", num),
            Self::NotFound => f.write_str("the specified entry is not found"),
            Self::InvalidDataOffset(num) => write!(f, "entry #{} has invalid data offset", num),
        }
    }
}
