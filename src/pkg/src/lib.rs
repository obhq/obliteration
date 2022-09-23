use self::entry::Entry;
use self::header::Header;
use self::param::Param;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use context::Context;
use sha2::Digest;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr::null_mut;
use util::mem::uninit;

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
pub extern "C" fn pkg_dump_entry(pkg: &Pkg, id: u32, file: *const c_char) -> *mut c_char {
    let file = util::str::from_c_unchecked(file);

    match pkg.dump_entry(id, file) {
        Ok(_) => null_mut(),
        Err(e) => util::str::to_c(&e.to_string()),
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

    pub fn dump_entry<F: AsRef<Path>>(&self, id: u32, file: F) -> Result<(), DumpEntryError> {
        // Find target entry.
        let (entry, index, data) = match self.find_entry(id) {
            Ok(v) => v,
            Err(e) => return Err(DumpEntryError::FindEntryFailed(e)),
        };

        // Open destination file.
        let mut file = match File::create(file) {
            Ok(v) => v,
            Err(e) => return Err(DumpEntryError::CreateDestinationFailed(e)),
        };

        if entry.is_encrypted() {
            if entry.key_index() != 3 || self.entry_key3.is_empty() {
                return Err(DumpEntryError::NoDecryptionKey(index));
            }

            // Decrypt and dump data.
            if let Err(e) = self.decrypt_entry_data(&entry, data, |b| file.write_all(&b)) {
                return Err(DumpEntryError::WriteDestinationFailed(e));
            }
        } else if let Err(e) = file.write_all(data) {
            return Err(DumpEntryError::WriteDestinationFailed(e));
        }

        Ok(())
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
                FindEntryError::NotFound => f.write_str("the package does not have param.sfo"),
                _ => e.fmt(f),
            },
            Self::ReadFailed(_) => f.write_str("the package has malformed param.sfo"),
        }
    }
}

#[derive(Debug)]
pub enum DumpEntryError {
    FindEntryFailed(FindEntryError),
    CreateDestinationFailed(std::io::Error),
    WriteDestinationFailed(std::io::Error),
    NoDecryptionKey(usize),
}

impl Error for DumpEntryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FindEntryFailed(e) => Some(e),
            Self::CreateDestinationFailed(e) | Self::WriteDestinationFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for DumpEntryError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::FindEntryFailed(e) => e.fmt(f),
            Self::CreateDestinationFailed(e) => e.fmt(f),
            Self::WriteDestinationFailed(e) => e.fmt(f),
            Self::NoDecryptionKey(num) => write!(f, "no decryption key for entry #{}", num),
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
