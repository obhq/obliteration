use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::{read_i64_le, read_u64_le};

pub(super) struct DynamicLinking {
    pltrelsz: u64,
    pltgot: u64,
    relasz: u64,
    relaent: u64,
    strsz: u64,
    syment: u64,
    pltrel: u64,
    fingerprint: u64,
    filename: u64,
    module_info: u64,
    hash: u64,
    jmprel: u64,
    rela: u64,
    strtab: u64,
    symtab: u64,
    hashsz: u64,
    symtabsz: u64,
}

impl DynamicLinking {
    pub const DT_NULL: i64 = 0;
    pub const DT_NEEDED: i64 = 1;
    pub const DT_PLTRELSZ: i64 = 2;
    pub const DT_PLTGOT: i64 = 3;
    pub const DT_RELA: i64 = 7;
    pub const DT_RELASZ: i64 = 8;
    pub const DT_RELAENT: i64 = 9;
    pub const DT_STRSZ: i64 = 10;
    pub const DT_SYMENT: i64 = 11;
    pub const DT_INIT: i64 = 12;
    pub const DT_FINI: i64 = 13;
    pub const DT_SONAME: i64 = 14;
    pub const DT_SYMBOLIC: i64 = 16;
    pub const DT_PLTREL: i64 = 20;
    pub const DT_DEBUG: i64 = 21;
    pub const DT_TEXTREL: i64 = 22;
    pub const DT_INIT_ARRAY: i64 = 25;
    pub const DT_FINI_ARRAY: i64 = 26;
    pub const DT_INIT_ARRAYSZ: i64 = 27;
    pub const DT_FINI_ARRAYSZ: i64 = 28;
    pub const DT_FLAGS: i64 = 30;
    pub const DT_PREINIT_ARRAY: i64 = 32;
    pub const DT_PREINIT_ARRAYSZ: i64 = 33;
    pub const DT_SCE_FINGERPRINT: i64 = 0x61000007;
    pub const DT_SCE_FILENAME: i64 = 0x61000009;
    pub const DT_SCE_MODULE_INFO: i64 = 0x6100000d;
    pub const DT_SCE_NEEDED_MODULE: i64 = 0x6100000f;
    pub const DT_SCE_MODULE_ATTR: i64 = 0x61000011;
    pub const DT_SCE_EXPORT_LIB: i64 = 0x61000013;
    pub const DT_SCE_IMPORT_LIB: i64 = 0x61000015;
    pub const DT_SCE_EXPORT_LIB_ATTR: i64 = 0x61000017;
    pub const DT_SCE_IMPORT_LIB_ATTR: i64 = 0x61000019;
    pub const DT_SCE_HASH: i64 = 0x61000025;
    pub const DT_SCE_PLTGOT: i64 = 0x61000027;
    pub const DT_SCE_JMPREL: i64 = 0x61000029;
    pub const DT_SCE_PLTREL: i64 = 0x6100002b;
    pub const DT_SCE_PLTRELSZ: i64 = 0x6100002d;
    pub const DT_SCE_RELA: i64 = 0x6100002f;
    pub const DT_SCE_RELASZ: i64 = 0x61000031;
    pub const DT_SCE_RELAENT: i64 = 0x61000033;
    pub const DT_SCE_STRTAB: i64 = 0x61000035;
    pub const DT_SCE_STRSZ: i64 = 0x61000037;
    pub const DT_SCE_SYMTAB: i64 = 0x61000039;
    pub const DT_SCE_SYMENT: i64 = 0x6100003b;
    pub const DT_SCE_HASHSZ: i64 = 0x6100003d;
    pub const DT_SCE_SYMTABSZ: i64 = 0x6100003f;

    pub fn parse(data: &[u8], _dynlib: &[u8]) -> Result<Self, ParseError> {
        // Simple check to see if data valid.
        if data.len() % 16 != 0 {
            return Err(ParseError::InvalidDataSize);
        }

        // Parse all dynamic linking data.
        let mut pltrelsz: Option<u64> = None;
        let mut pltgot: Option<u64> = None;
        let mut relasz: Option<u64> = None;
        let mut relaent: Option<u64> = None;
        let mut strsz: Option<u64> = None;
        let mut syment: Option<u64> = None;
        let mut pltrel: Option<u64> = None;
        let mut fingerprint: Option<u64> = None;
        let mut filename: Option<u64> = None;
        let mut module_info: Option<u64> = None;
        let mut hash: Option<u64> = None;
        let mut jmprel: Option<u64> = None;
        let mut rela: Option<u64> = None;
        let mut strtab: Option<u64> = None;
        let mut symtab: Option<u64> = None;
        let mut hashsz: Option<u64> = None;
        let mut symtabsz: Option<u64> = None;
        let mut offset = 0;

        while offset < data.len() {
            // Read fields.
            let data = unsafe { data.as_ptr().add(offset) };
            let tag = unsafe { read_i64_le(data, 0) };
            let value = unsafe { read_u64_le(data, 8) };

            // Parse entry.
            match tag {
                Self::DT_NULL => break,
                Self::DT_NEEDED => {}
                Self::DT_PLTRELSZ | Self::DT_SCE_PLTRELSZ => pltrelsz = Some(value),
                Self::DT_PLTGOT | Self::DT_SCE_PLTGOT => pltgot = Some(value),
                Self::DT_RELASZ | Self::DT_SCE_RELASZ => relasz = Some(value),
                Self::DT_RELAENT | Self::DT_SCE_RELAENT => relaent = Some(value),
                Self::DT_STRSZ | Self::DT_SCE_STRSZ => strsz = Some(value),
                Self::DT_SYMENT | Self::DT_SCE_SYMENT => syment = Some(value),
                Self::DT_INIT => {}
                Self::DT_FINI => {}
                Self::DT_SONAME => {}
                Self::DT_SYMBOLIC => {}
                Self::DT_PLTREL | Self::DT_SCE_PLTREL => pltrel = Some(value),
                Self::DT_DEBUG => {}
                Self::DT_TEXTREL => {}
                Self::DT_INIT_ARRAY => {}
                Self::DT_FINI_ARRAY => {}
                Self::DT_INIT_ARRAYSZ => {}
                Self::DT_FINI_ARRAYSZ => {}
                Self::DT_FLAGS => {}
                Self::DT_PREINIT_ARRAY => {}
                Self::DT_PREINIT_ARRAYSZ => {}
                Self::DT_SCE_FINGERPRINT => fingerprint = Some(value),
                Self::DT_SCE_FILENAME => filename = Some(value),
                Self::DT_SCE_MODULE_INFO => module_info = Some(value),
                Self::DT_SCE_NEEDED_MODULE => {}
                Self::DT_SCE_MODULE_ATTR => {}
                Self::DT_SCE_EXPORT_LIB => {}
                Self::DT_SCE_IMPORT_LIB => {}
                Self::DT_SCE_EXPORT_LIB_ATTR => {}
                Self::DT_SCE_IMPORT_LIB_ATTR => {}
                Self::DT_SCE_HASH => hash = Some(value),
                Self::DT_SCE_JMPREL => jmprel = Some(value),
                Self::DT_SCE_RELA => rela = Some(value),
                Self::DT_SCE_STRTAB => strtab = Some(value),
                Self::DT_SCE_SYMTAB => symtab = Some(value),
                Self::DT_SCE_HASHSZ => hashsz = Some(value),
                Self::DT_SCE_SYMTABSZ => symtabsz = Some(value),
                _ => return Err(ParseError::UnknownTag(tag)),
            }

            offset += 16;
        }

        // Construct instance.
        Ok(Self {
            pltrelsz: pltrelsz.ok_or(ParseError::NoPltrelsz)?,
            pltgot: pltgot.ok_or(ParseError::NoPltgot)?,
            relasz: relasz.ok_or(ParseError::NoRelasz)?,
            relaent: relaent.ok_or(ParseError::NoRelaent)?,
            strsz: strsz.ok_or(ParseError::NoStrsz)?,
            syment: syment.ok_or(ParseError::NoSyment)?,
            pltrel: pltrel.ok_or(ParseError::NoPltrel)?,
            fingerprint: fingerprint.ok_or(ParseError::NoFingerprint)?,
            filename: filename.ok_or(ParseError::NoFilename)?,
            module_info: module_info.ok_or(ParseError::NoModuleInfo)?,
            hash: hash.ok_or(ParseError::NoHash)?,
            jmprel: jmprel.ok_or(ParseError::NoJmprel)?,
            rela: rela.ok_or(ParseError::NoRela)?,
            strtab: strtab.ok_or(ParseError::NoStrtab)?,
            symtab: symtab.ok_or(ParseError::NoSymtab)?,
            hashsz: hashsz.ok_or(ParseError::NoHashsz)?,
            symtabsz: symtabsz.ok_or(ParseError::NoSymtabsz)?,
        })
    }

    pub fn relaent(&self) -> u64 {
        self.relaent
    }

    pub fn syment(&self) -> u64 {
        self.syment
    }

    pub fn pltrel(&self) -> u64 {
        self.pltrel
    }
}

#[derive(Debug)]
pub enum ParseError {
    InvalidDataSize,
    NoPltrelsz,
    NoPltgot,
    NoRelasz,
    NoRelaent,
    NoStrsz,
    NoSyment,
    NoPltrel,
    NoFingerprint,
    NoFilename,
    NoModuleInfo,
    NoHash,
    NoJmprel,
    NoRela,
    NoStrtab,
    NoSymtab,
    NoHashsz,
    NoSymtabsz,
    UnknownTag(i64),
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidDataSize => f.write_str("invalid data size"),
            Self::NoPltrelsz => f.write_str("entry DT_PLTRELSZ or DT_SCE_PLTRELSZ does not exists"),
            Self::NoPltgot => f.write_str("entry DT_PLTGOT or DT_SCE_PLTGOT does not exists"),
            Self::NoRelasz => f.write_str("entry DT_RELASZ or DT_SCE_RELASZ does not exists"),
            Self::NoRelaent => f.write_str("entry DT_RELAENT or DT_SCE_RELAENT does not exists"),
            Self::NoStrsz => f.write_str("entry DT_STRSZ or DT_SCE_STRSZ does not exists"),
            Self::NoSyment => f.write_str("entry DT_SYMENT or DT_SCE_SYMENT does not exists"),
            Self::NoPltrel => f.write_str("entry DT_PLTREL or DT_SCE_PLTREL does not exists"),
            Self::NoFingerprint => f.write_str("entry DT_SCE_FINGERPRINT does not exists"),
            Self::NoFilename => f.write_str("entry DT_SCE_FILENAME does not exists"),
            Self::NoModuleInfo => f.write_str("entry DT_SCE_MODULE_INFO does not exists"),
            Self::NoHash => f.write_str("entry DT_SCE_HASH does not exists"),
            Self::NoJmprel => f.write_str("entry DT_SCE_JMPREL does not exists"),
            Self::NoRela => f.write_str("entry DT_SCE_RELA does not exists"),
            Self::NoStrtab => f.write_str("entry DT_SCE_STRTAB does not exists"),
            Self::NoSymtab => f.write_str("entry DT_SCE_SYMTAB does not exists"),
            Self::NoHashsz => f.write_str("entry DT_SCE_HASHSZ does not exists"),
            Self::NoSymtabsz => f.write_str("entry DT_SCE_SYMTABSZ does not exists"),
            Self::UnknownTag(t) => write!(f, "unknown tag {:#018x}", t),
        }
    }
}
