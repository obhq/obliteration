use self::entry::Entry;
use self::header::Header;
use self::param::Param;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use context::Context;
use sha2::Digest;
use std::error::Error;
use std::ffi::c_void;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use util::mem::{new_buffer, uninit};

pub mod entry;
pub mod header;
pub mod param;

#[no_mangle]
pub extern "C" fn pkg_open<'c>(
    ctx: &'c Context,
    file: *const c_char,
    error: *mut *mut c_char,
) -> *mut Pkg<'c> {
    let path = util::str::from_c_unchecked(file);
    let pkg = match Pkg::open(ctx, path) {
        Ok(v) => Box::new(v),
        Err(e) => {
            util::str::set_c(error, &e.to_string());
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
pub extern "C" fn pkg_dump_entries(pkg: &Pkg, dir: *const c_char) -> *mut error::Error {
    let dir = util::str::from_c_unchecked(dir);

    match pkg.dump_entries(dir) {
        Ok(_) => null_mut(),
        Err(e) => error::Error::new(&e),
    }
}

#[no_mangle]
pub extern "C" fn pkg_dump_pfs(
    pkg: &Pkg,
    dir: *const c_char,
    status: extern "C" fn(u64, u64, *const c_char, *mut c_void),
    ud: *mut c_void,
) -> *mut error::Error {
    if let Err(e) = pkg.dump_pfs(util::str::from_c_unchecked(dir), status, ud) {
        return error::Error::new(&e);
    }

    null_mut()
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
pub struct Pkg<'c> {
    ctx: &'c Context,
    raw: memmap2::Mmap,
    header: Header,
    entry_key3: Vec<u8>,
    ekpfs: Vec<u8>,
}

impl<'c> Pkg<'c> {
    pub fn open<P: AsRef<Path>>(ctx: &'c Context, path: P) -> Result<Self, OpenError> {
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
            ctx,
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

    pub fn dump_entries<D: AsRef<Path>>(&self, dir: D) -> Result<(), DumpEntryError> {
        let dir = dir.as_ref();

        for num in 0..self.header.entry_count() {
            // Check offset.
            let offset = self.header.table_offset() + num * Entry::RAW_SIZE;
            let raw = match self.raw.get(offset..(offset + Entry::RAW_SIZE)) {
                Some(v) => v.as_ptr(),
                None => return Err(DumpEntryError::InvalidEntryOffset(num)),
            };

            // Read entry.
            let entry = Entry::read(raw);

            // Skip all entries that we don't have decryption key. We have decryption key for all
            // required entries so it is safe to skip one that we don't have.
            if entry.is_encrypted() {
                if entry.key_index() != 3 {
                    continue;
                } else if self.entry_key3.is_empty() {
                    return Err(DumpEntryError::NoDecryptionKey(num));
                }
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
                None => return Err(DumpEntryError::InvalidDataOffset(num)),
            };

            // Get destination path.
            let mut path = dir.to_path_buf();

            path.push(format!("entry_{}", entry.id()));

            // Open destination file.
            let mut file = match File::create(&path) {
                Ok(v) => v,
                Err(e) => return Err(DumpEntryError::CreateDestinationFailed(path, e)),
            };

            // Dump entry data.
            if entry.is_encrypted() {
                if let Err(e) = self.decrypt_entry_data(&entry, data, |b| file.write_all(&b)) {
                    return Err(DumpEntryError::WriteDestinationFailed(path, e));
                }
            } else if let Err(e) = file.write_all(data) {
                return Err(DumpEntryError::WriteDestinationFailed(path, e));
            }
        }

        Ok(())
    }

    pub fn dump_pfs<O: AsRef<Path>>(
        &self,
        output: O,
        status: extern "C" fn(u64, u64, *const c_char, *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), DumpPfsError> {
        // Get outer PFS.
        let image_offset = self.header.pfs_offset();
        let image_size = self.header.pfs_size();
        let image = match self.raw.get(image_offset..(image_offset + image_size)) {
            Some(v) => v,
            None => return Err(DumpPfsError::InvalidOuterOffset),
        };

        // Mount outer PFS.
        let image = match pfs::open(image, Some(&self.ekpfs)) {
            Ok(v) => v,
            Err(e) => return Err(DumpPfsError::OpenOuterFailed(e)),
        };

        let pfs = match pfs::mount(image.as_ref()) {
            Ok(v) => v,
            Err(e) => return Err(DumpPfsError::MountOuterFailed(e)),
        };

        let super_root = match pfs.open_super_root() {
            Ok(v) => v,
            Err(e) => return Err(DumpPfsError::OpenSuperRootFailed(e)),
        };

        // Dump files.
        self.dump_pfs_directory(Vec::new(), super_root, output, status, ud)
    }

    fn dump_pfs_directory<O: AsRef<Path>>(
        &self,
        path: Vec<&[u8]>,
        dir: pfs::directory::Directory,
        output: O,
        status: extern "C" fn(u64, u64, *const c_char, *mut c_void),
        ud: *mut c_void,
    ) -> Result<(), DumpPfsError> {
        // Open PFS directory.
        let items = match dir.open() {
            Ok(v) => v,
            Err(e) => {
                return Err(DumpPfsError::OpenDirectoryFailed(
                    self.build_pfs_path(&path),
                    e,
                ));
            }
        };

        // Enumerate items.
        let mut buffer: Vec<u8> = new_buffer(32768);

        for (name, item) in items {
            // Build destination path.
            let mut output: PathBuf = output.as_ref().into();

            match std::str::from_utf8(&name) {
                Ok(v) => output.push(v),
                Err(_) => {
                    return Err(DumpPfsError::UnsupportedFileName(
                        self.build_pfs_path(&path),
                    ));
                }
            }

            // Build source path.
            let mut path = path.clone();

            path.push(&name);

            // Handle item.
            match item {
                pfs::directory::Item::Directory(i) => {
                    // Create output directory.
                    if let Err(e) = std::fs::create_dir(&output) {
                        return Err(DumpPfsError::CreateDirectoryFailed(output, e));
                    }

                    // Dump.
                    self.dump_pfs_directory(path, i, &output, status, ud)?;
                }
                pfs::directory::Item::File(mut i) => {
                    // Check if file is compressed.
                    let mut pfsc;
                    let (size, file): (u64, &mut dyn Read) = if i.is_compressed() {
                        pfsc = match pfs::pfsc::Reader::open(i) {
                            Ok(v) => v,
                            Err(e) => {
                                return Err(DumpPfsError::CreateDecompressorFailed(output, e));
                            }
                        };

                        (pfsc.len(), &mut pfsc)
                    } else {
                        (i.len(), &mut i)
                    };

                    // Report initial status.
                    let mut status_name = name.clone();

                    status_name.push(0);

                    (status)(0, size, status_name.as_ptr() as _, ud);

                    // Open destination file.
                    let mut dest = std::fs::OpenOptions::new();

                    dest.create_new(true);
                    dest.write(true);

                    let mut dest = match dest.open(&output) {
                        Ok(v) => v,
                        Err(e) => return Err(DumpPfsError::CreateFileFailed(output, e)),
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
                                    return Err(DumpPfsError::ReadFileFailed(
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
                            return Err(DumpPfsError::WriteFileFailed(output, e));
                        }

                        written += read as u64; // Buffer size just 32768.

                        // Update status.
                        (status)(written, size, status_name.as_ptr() as _, ud);
                    }
                }
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
        let fake_key = self.ctx.fake_pfs_key();

        self.ekpfs = match fake_key.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, &encrypted) {
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
        let key3 = self.ctx.pkg_key3();

        self.entry_key3 = match key3.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, &keys[3]) {
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

#[derive(Debug)]
pub enum DumpEntryError {
    InvalidEntryOffset(usize),
    InvalidDataOffset(usize),
    CreateDestinationFailed(PathBuf, std::io::Error),
    WriteDestinationFailed(PathBuf, std::io::Error),
    NoDecryptionKey(usize),
}

impl Error for DumpEntryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDestinationFailed(_, e) | Self::WriteDestinationFailed(_, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for DumpEntryError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidEntryOffset(i) => write!(f, "entry #{} has invalid offset", i),
            Self::InvalidDataOffset(i) => write!(f, "entry #{} has invalid data offset", i),
            Self::CreateDestinationFailed(p, _) => write!(f, "cannot create {}", p.display()),
            Self::WriteDestinationFailed(p, _) => write!(f, "cannot write {}", p.display()),
            Self::NoDecryptionKey(i) => write!(f, "no decryption key for entry #{}", i),
        }
    }
}

#[derive(Debug)]
pub enum DumpPfsError {
    InvalidOuterOffset,
    OpenOuterFailed(pfs::OpenError),
    MountOuterFailed(pfs::MountError),
    OpenSuperRootFailed(pfs::OpenSuperRootError),
    OpenDirectoryFailed(String, pfs::directory::OpenError),
    UnsupportedFileName(String),
    CreateDirectoryFailed(PathBuf, std::io::Error),
    CreateDecompressorFailed(PathBuf, pfs::pfsc::OpenError),
    CreateFileFailed(PathBuf, std::io::Error),
    ReadFileFailed(String, std::io::Error),
    WriteFileFailed(PathBuf, std::io::Error),
}

impl Error for DumpPfsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenOuterFailed(e) => Some(e),
            Self::MountOuterFailed(e) => Some(e),
            Self::OpenSuperRootFailed(e) => Some(e),
            Self::OpenDirectoryFailed(_, e) => Some(e),
            Self::CreateDirectoryFailed(_, e) => Some(e),
            Self::CreateDecompressorFailed(_, e) => Some(e),
            Self::CreateFileFailed(_, e) => Some(e),
            Self::ReadFileFailed(_, e) => Some(e),
            Self::WriteFileFailed(_, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for DumpPfsError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidOuterOffset => f.write_str("invalid offset for outer PFS"),
            Self::OpenOuterFailed(_) => f.write_str("cannot open outer PFS"),
            Self::MountOuterFailed(_) => f.write_str("cannot mount outer PFS"),
            Self::OpenSuperRootFailed(_) => f.write_str("cannot open super-root"),
            Self::OpenDirectoryFailed(p, _) => write!(f, "cannot open {}", p),
            Self::UnsupportedFileName(p) => {
                write!(f, "directory {} has file(s) with unsupported name", p)
            }
            Self::CreateDirectoryFailed(p, _) => {
                write!(f, "cannot create directory {}", p.display())
            }
            Self::CreateDecompressorFailed(p, _) => {
                write!(f, "cannot create decompressor for {}", p.display())
            }
            Self::CreateFileFailed(p, _) => write!(f, "cannot create {}", p.display()),
            Self::ReadFileFailed(p, _) => write!(f, "cannot read {}", p),
            Self::WriteFileFailed(p, _) => write!(f, "cannot write {}", p.display()),
        }
    }
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
