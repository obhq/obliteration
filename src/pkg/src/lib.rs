use self::entry::{Entry, EntryKey};
use self::header::Header;
use self::param::Param;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use context::Context;
use sha2::Digest;
use std::error::Error;
use std::ffi::c_void;
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
pub extern "C" fn pkg_enum_entries(
    pkg: &Pkg,
    cb: extern "C" fn(&Entry, usize, *mut c_void) -> *mut c_void,
    ud: *mut c_void,
) -> *mut c_void {
    let header = pkg.header();
    let table = pkg.raw()[header.table_offset()..].as_ptr();

    for i in 0..header.entry_count() {
        // Read entry.
        let entry = Entry::read(pkg, table, i);
        let public = match entry.id() {
            Entry::PARAM_SFO => true,
            Entry::PIC1_PNG => true,
            Entry::ICON0_PNG => true,
            _ => false,
        };

        if !public {
            continue;
        }

        // Invoke callback.
        let result = cb(&entry, i, ud);

        if !result.is_null() {
            return result;
        }
    }

    null_mut()
}

#[no_mangle]
pub extern "C" fn pkg_entry_id(entry: &Entry) -> u32 {
    entry.id()
}

#[no_mangle]
pub extern "C" fn pkg_entry_dump(entry: &Entry, file: *const c_char) -> *mut c_char {
    // Open destination file.
    let mut dest = match File::create(util::str::from_c_unchecked(file)) {
        Ok(v) => v,
        Err(e) => return util::str::to_c(&e.to_string()),
    };

    // Write destination file.
    let pkg = entry.pkg();
    let offset = entry.offset();

    if entry.is_encrypted() {
        if entry.key_index() != 3 {
            return util::str::to_c("no decryption key for the entry");
        }

        // Get encrypted data.
        let size = (entry.size() + 15) & !15; // We need to include padding.
        let encrypted = match pkg.raw().get(offset..(offset + size)) {
            Some(v) => v.as_ptr(),
            None => return util::str::to_c("invalid data offset"),
        };

        // Get secret seed.
        let mut secret_seed = Vec::from(entry.to_bytes());

        match pkg.entry_key() {
            Some(k) => {
                let ctx = pkg.context();
                let key3 = ctx.pkg_key3();

                match key3.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, &k.keys()[3]) {
                    Ok(v) => secret_seed.extend(v),
                    Err(e) => return util::str::to_c(&e.to_string()),
                }
            }
            None => return util::str::to_c("no decryption key for the entry"),
        }

        // Calculate secret.
        let mut hasher = sha2::Sha256::new();

        hasher.update(secret_seed.as_slice());

        let secret = hasher.finalize();
        let first = (&secret[..16]).as_ptr();
        let last = (&secret[16..]).as_ptr();
        let mut iv: [u8; 16] = uninit();
        let mut key: [u8; 16] = uninit();

        unsafe { first.copy_to_nonoverlapping(iv.as_mut_ptr(), 16) };
        unsafe { last.copy_to_nonoverlapping(key.as_mut_ptr(), 16) };

        // Dump content.
        let mut decryptor = cbc::Decryptor::<aes::Aes128>::new(&key.into(), &iv.into());
        let mut written = 0;

        while written < size {
            // Decrypt.
            let mut block: [u8; 16] = uninit();
            let source = unsafe { encrypted.offset(written as _) };

            unsafe { source.copy_to_nonoverlapping(block.as_mut_ptr(), 16) };

            decryptor.decrypt_block_mut(&mut block.into());

            // Write file.
            if let Err(e) = dest.write_all(&block) {
                return util::str::to_c(&e.to_string());
            }

            written += 16;
        }
    } else {
        let data = match pkg.raw().get(offset..(offset + entry.size())) {
            Some(v) => v,
            None => return util::str::to_c("invalid data offset"),
        };

        if let Err(e) = dest.write_all(data) {
            return util::str::to_c(&e.to_string());
        }
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
    entry_key: Option<EntryKey>,
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

        // Check file with the header.
        let table_offset = header.table_offset();

        if raw.len() < (table_offset + Entry::RAW_SIZE * header.entry_count()) {
            return Err(OpenError::InvalidTableOffset);
        }

        let mut pkg = Self {
            ctx,
            raw,
            header,
            entry_key: None,
        };

        // Read keys entry.
        let table = unsafe { pkg.raw.as_ptr().offset(table_offset as _) };

        for i in 0..pkg.header.entry_count() {
            // Check if entry is a keys entry.
            let entry = Entry::read(&pkg, table, i);

            if entry.id() != Entry::KEYS {
                continue;
            }

            // Slice entry data.
            let offset = entry.offset();
            let mut data = match pkg.raw.get(offset..(offset + (32 + 32 * 7 + 256 * 7))) {
                Some(v) => v.as_ptr(),
                None => return Err(OpenError::InvalidKeysOffset),
            };

            drop(entry);

            // Read keys.
            let mut seed: [u8; 32] = uninit();
            let mut digests: [[u8; 32]; 7] = uninit();
            let mut keys: [[u8; 256]; 7] = uninit();

            unsafe { data.copy_to_nonoverlapping(seed.as_mut_ptr(), 32) };
            unsafe { data = data.offset(32) };

            for i in 0..7 {
                unsafe { data.copy_to_nonoverlapping(digests[i].as_mut_ptr(), 32) };
                unsafe { data = data.offset(32) };
            }

            for i in 0..7 {
                unsafe { data.copy_to_nonoverlapping(keys[i].as_mut_ptr(), 256) };
                unsafe { data = data.offset(256) };
            }

            pkg.entry_key = Some(EntryKey::new(seed, digests, keys));
            break;
        }

        Ok(pkg)
    }

    pub fn context(&self) -> &'c Context {
        self.ctx
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn raw(&self) -> &[u8] {
        self.raw.as_ref()
    }

    pub fn entry_key(&self) -> Option<&EntryKey> {
        self.entry_key.as_ref()
    }
}

#[derive(Debug)]
pub enum OpenError {
    OpenFailed(std::io::Error),
    MapFailed(std::io::Error),
    InvalidHeader,
    InvalidTableOffset,
    InvalidKeysOffset,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OpenError::OpenFailed(e) => Some(e),
            OpenError::MapFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            OpenError::OpenFailed(e) => e.fmt(f),
            OpenError::MapFailed(e) => e.fmt(f),
            OpenError::InvalidHeader => f.write_str("invalid PKG header"),
            OpenError::InvalidTableOffset => f.write_str("invalid PKG table offset"),
            OpenError::InvalidKeysOffset => f.write_str("invalid PKG keys offset"),
        }
    }
}
