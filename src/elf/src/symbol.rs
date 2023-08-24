use crate::{FileInfo, StringTableError};
use byteorder::{ByteOrder, LE};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
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

    /// Thread local data object.
    pub const STT_TLS: u8 = 6;

    /// PS4 specific.
    pub const STT_ENTRY: u8 = 11;

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

    pub fn decode_name(&self) -> Option<(&str, u16, u16)> {
        // Extract local name.
        let (name, remain) = match self.name.find('#') {
            Some(v) => (&self.name[..v], &self.name[(v + 1)..]),
            None => return None,
        };

        if name.is_empty() {
            return None;
        }

        // Extract library ID and module ID.
        let (lib_id, mod_id) = match remain.find('#') {
            Some(v) => (&remain[..v], &remain[(v + 1)..]),
            None => return None,
        };

        if lib_id.is_empty() || mod_id.is_empty() {
            return None;
        }

        // Decode module ID and library ID.
        let mod_id = Self::decode_id(mod_id)?;
        let lib_id = Self::decode_id(lib_id)?;

        Some((name, lib_id, mod_id))
    }

    fn decode_id(v: &str) -> Option<u16> {
        if v.len() > 3 {
            return None;
        }

        let s = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
        let mut r = 0u32;

        for c in v.chars() {
            r <<= 6;
            r |= s.find(c)? as u32;
        }

        if r > u16::MAX as _ {
            None
        } else {
            Some(r as u16)
        }
    }
}

/// A decoded symbol name.
#[derive(PartialEq)]
pub enum SymbolName<'a> {
    Simple(Cow<'a, str>),
    Sony(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),
}

impl<'a> SymbolName<'a> {
    pub fn build_value(&self) -> Cow<'_, str> {
        match self {
            Self::Simple(n) => Cow::Borrowed(n.as_ref()),
            Self::Sony(n, l, m) => format!("{}#{}#{}", n, l, m).into(),
        }
    }

    pub fn calculate_hash(&self) -> u32 {
        let mut h = 0;

        match self {
            Self::Simple(n) => Self::elf_hash(&mut h, &n),
            Self::Sony(n, l, m) => {
                Self::elf_hash(&mut h, &n);
                Self::elf_hash(&mut h, "#");
                Self::elf_hash(&mut h, &l);
                Self::elf_hash(&mut h, "#");
                Self::elf_hash(&mut h, &m);
            }
        }

        h
    }

    fn elf_hash(h: &mut u32, s: &str) {
        let mut g;

        for b in s.bytes() {
            *h = (*h << 4) + (b as u32);
            g = *h & 0xf0000000;
            if g != 0 {
                *h ^= g >> 24;
            }
            *h &= !g;
        }
    }
}

impl<'a> Display for SymbolName<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.build_value())
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
