use super::{FileInfo, StringTableError};
use byteorder::{ByteOrder, LE};
use thiserror::Error;

/// An iterator over the `Elf64_Sym`.
pub struct Symbols<'a> {
    next: &'a [u8],
    info: &'a FileInfo,
}

impl<'a> Symbols<'a> {
    pub(crate) fn new(next: &'a [u8], info: &'a FileInfo) -> Self {
        Self { next, info }
    }
}

impl<'a> Iterator for Symbols<'a> {
    type Item = Result<Symbol, ReadSymbolError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if all entries has been read.
        if self.next.is_empty() {
            return None;
        } else if self.next.len() < 24 {
            return Some(Err(ReadSymbolError::InvalidEntry));
        }

        // Read the entry.
        let name = LE::read_u32(self.next);
        let info = self.next[4];
        let shndx = LE::read_u16(&self.next[6..]);
        let value = LE::read_u64(&self.next[8..]);

        // Load name.
        let name = match self.info.read_str(name.try_into().unwrap()) {
            Ok(v) => v.to_owned(),
            Err(e) => return Some(Err(ReadSymbolError::InvalidNameOffset(name, e))),
        };

        // Move to next entry.
        self.next = &self.next[24..];

        Some(Ok(Symbol {
            name,
            info,
            shndx,
            value: value.try_into().unwrap(),
        }))
    }
}

/// Represents an `Elf64_Sym`.
#[derive(Debug)]
pub struct Symbol {
    name: String,
    info: u8,
    shndx: u16,
    value: usize,
}

impl Symbol {
    /// Symbol's type is not specified.
    pub const STT_NOTYPE: u8 = 0;

    /// Symbol is a data object (variable, array, etc.)
    pub const STT_OBJECT: u8 = 1;

    /// Symbol is executable code (function, etc.)
    pub const STT_FUNC: u8 = 2;

    /// Symbol refers to a section.
    pub const STT_SECTION: u8 = 3;

    /// Symbol gives the name of the source file associated with the object file.
    pub const STT_FILE: u8 = 4;

    /// Symbol labels an uninitialized common block (Treated same as STT_OBJECT)
    pub const STT_COMMON: u8 = 5;

    /// Thread local data object.
    pub const STT_TLS: u8 = 6;

    /// Lowest operating system-specific symbol type
    pub const STT_LOOS: u8 = 10;

    /// PS4 specific.
    pub const STT_ENTRY: u8 = 11;

    /// Highest operating system-specific symbol type
    pub const STT_HIOS: u8 = 12;

    /// Local symbol, not visible outside obj file containing def.
    pub const STB_LOCAL: u8 = 0;

    /// Global symbol, visible to all object files being combined.
    pub const STB_GLOBAL: u8 = 1;

    /// Weak symbol, like global but lower-precedence.
    pub const STB_WEAK: u8 = 2;

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn ty(&self) -> u8 {
        self.info & 0xf
    }

    pub fn binding(&self) -> u8 {
        self.info >> 4
    }

    pub fn shndx(&self) -> u16 {
        self.shndx
    }

    pub fn value(&self) -> usize {
        self.value
    }
}

/// Represents an error when reading `Elf64_Sym` is failed.
#[derive(Debug, Error)]
pub enum ReadSymbolError {
    #[error("the entry is not a valid symbol entry")]
    InvalidEntry,

    #[error("name offset {0} is not valid")]
    InvalidNameOffset(u32, #[source] StringTableError),
}
