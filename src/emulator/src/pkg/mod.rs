use self::header::Header;
use crate::context::Context;
use crate::util::binary::{read_u32_be, write_u32_be};
use crate::util::mem::uninit;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::Path;

pub mod header;
pub mod keys;

// https://www.psdevwiki.com/ps4/Package_Files
pub struct PkgFile<'c> {
    ctx: &'c Context,
    raw: memmap2::Mmap,
    header: Header,
    entry_key: Option<PkgEntryKey>,
}

impl<'c> PkgFile<'c> {
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
        if raw.len() < (header.table_offset() + PkgEntry::RAW_SIZE * header.entry_count()) {
            return Err(OpenError::InvalidTableOffset);
        }

        // Read keys entry.
        let mut pkg = Self {
            ctx,
            raw,
            header,
            entry_key: None,
        };

        for i in 0..pkg.header.entry_count() {
            // Check if entry is a keys entry.
            let entry = PkgEntry::read(&pkg, pkg.raw.as_ptr(), i);

            if entry.id() != PkgEntry::KEYS {
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

            pkg.entry_key = Some(PkgEntryKey::new(seed, digests, keys));
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

    pub fn entry_key(&self) -> Option<&PkgEntryKey> {
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

pub struct PkgEntry<'c, 'o> {
    owner: &'o PkgFile<'c>,
    id: u32,
    filename_offset: u32,
    flags1: u32,
    flags2: u32,
    offset: u32,
    size: u32,
}

impl<'c, 'o> PkgEntry<'c, 'o> {
    pub const RAW_SIZE: usize = 32;

    pub const KEYS: u32 = 0x00000010;
    pub const PARAM_SFO: u32 = 0x00001000;
    pub const PIC1_PNG: u32 = 0x00001006;
    pub const ICON0_PNG: u32 = 0x00001200;

    pub fn read(owner: &'o PkgFile<'c>, table: *const u8, index: usize) -> Self {
        let raw = unsafe { table.offset((index * Self::RAW_SIZE) as _) };
        let id = read_u32_be(raw, 0);
        let filename_offset = read_u32_be(raw, 4);
        let flags1 = read_u32_be(raw, 8);
        let flags2 = read_u32_be(raw, 12);
        let offset = read_u32_be(raw, 16);
        let size = read_u32_be(raw, 20);

        Self {
            owner,
            id,
            filename_offset,
            flags1,
            flags2,
            offset,
            size,
        }
    }

    pub fn owner(&self) -> &'o PkgFile<'c> {
        self.owner
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_encrypted(&self) -> bool {
        self.flags1 & 0x80000000 != 0
    }

    pub fn key_index(&self) -> usize {
        ((self.flags2 & 0xf000) >> 12) as _
    }

    pub fn offset(&self) -> usize {
        self.offset as _
    }

    pub fn size(&self) -> usize {
        self.size as _
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let p = buf.as_mut_ptr();

        write_u32_be(p, 0, self.id);
        write_u32_be(p, 4, self.filename_offset);
        write_u32_be(p, 8, self.flags1);
        write_u32_be(p, 12, self.flags2);
        write_u32_be(p, 16, self.offset);
        write_u32_be(p, 20, self.size);

        buf
    }
}

pub struct PkgEntryKey {
    seed: [u8; 32],
    digests: [[u8; 32]; 7],
    keys: [[u8; 256]; 7],
}

impl PkgEntryKey {
    pub fn new(seed: [u8; 32], digests: [[u8; 32]; 7], keys: [[u8; 256]; 7]) -> Self {
        Self {
            seed,
            digests,
            keys,
        }
    }

    pub fn keys(&self) -> [[u8; 256]; 7] {
        self.keys
    }
}
