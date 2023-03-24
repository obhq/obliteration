use byteorder::{ByteOrder, LE};
use thiserror::Error;

/// Contains data required for dynamic linking.
pub struct DynamicLinking {
    pltrelsz: u64,
    pltgot: u64,
    relasz: u64,
    relaent: u64,
    syment: u64,
    pltrel: u64,
    fingerprint: u64,
    filename: u64,
    module_info: ModuleInfo,
    needed_modules: Vec<ModuleInfo>,
    exports: Vec<LibraryInfo>,
    imports: Vec<LibraryInfo>,
    hash: u64,
    jmprel: u64,
    rela: u64,
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

    pub(super) fn parse(data: &[u8], dynlib: &[u8]) -> Result<Self, ParseError> {
        // Simple check to see if data valid.
        if data.len() % 16 != 0 {
            return Err(ParseError::InvalidDataSize);
        }

        // Find the offset of data tables.
        let mut strtab: Option<u64> = None;
        let mut strsz: Option<u64> = None;
        let mut offset = 0;

        while offset < data.len() {
            // Read fields.
            let data = &data[offset..(offset + 16)];
            let tag = LE::read_i64(&data);
            let value = &data[8..];

            // Parse entry.
            match tag {
                Self::DT_SCE_STRTAB => strtab = Some(LE::read_u64(value)),
                Self::DT_STRSZ | Self::DT_SCE_STRSZ => strsz = Some(LE::read_u64(value)),
                _ => {}
            }

            offset += 16;
        }

        let strtab = strtab.ok_or(ParseError::NoStrtab)? as usize;
        let strsz = strsz.ok_or(ParseError::NoStrsz)? as usize;

        // Get data tables.
        let strtab = match dynlib.get(strtab..(strtab + strsz)) {
            Some(v) => v,
            None => return Err(ParseError::InvalidStrtab),
        };

        let get_str = |offset: usize| -> Option<String> {
            let raw = match strtab.get(offset..) {
                Some(v) => match v.iter().position(|&b| b == 0) {
                    Some(i) => &v[..i],
                    None => return None,
                },
                None => return None,
            };

            match std::str::from_utf8(raw) {
                Ok(v) => Some(v.to_owned()),
                Err(_) => None,
            }
        };

        let parse_module_info = |value: &[u8]| -> Option<ModuleInfo> {
            let name = LE::read_u32(&value) as usize;
            let version_minor = value[4];
            let version_major = value[5];
            let id = LE::read_u16(&value[6..]);

            Some(ModuleInfo {
                id,
                name: get_str(name)?,
                version_major,
                version_minor,
            })
        };

        let parse_library_info = |value: &[u8]| -> Option<LibraryInfo> {
            let name = LE::read_u32(value) as usize;
            let version = LE::read_u16(&value[4..]);
            let id = LE::read_u16(&value[6..]);

            Some(LibraryInfo {
                id,
                name: get_str(name)?,
                version,
            })
        };

        // Parse all dynamic linking data.
        let mut pltrelsz: Option<u64> = None;
        let mut pltgot: Option<u64> = None;
        let mut relasz: Option<u64> = None;
        let mut relaent: Option<u64> = None;
        let mut syment: Option<u64> = None;
        let mut pltrel: Option<u64> = None;
        let mut fingerprint: Option<u64> = None;
        let mut filename: Option<u64> = None;
        let mut module_info: Option<ModuleInfo> = None;
        let mut needed_modules: Vec<ModuleInfo> = Vec::new();
        let mut exports: Vec<LibraryInfo> = Vec::new();
        let mut imports: Vec<LibraryInfo> = Vec::new();
        let mut hash: Option<u64> = None;
        let mut jmprel: Option<u64> = None;
        let mut rela: Option<u64> = None;
        let mut symtab: Option<u64> = None;
        let mut hashsz: Option<u64> = None;
        let mut symtabsz: Option<u64> = None;
        let mut offset = 0;
        let mut index = 0;

        while offset < data.len() {
            // Read fields.
            let data = &data[offset..(offset + 16)];
            let tag = LE::read_i64(&data);
            let value = &data[8..];

            // Parse entry.
            match tag {
                Self::DT_NULL => break,
                Self::DT_NEEDED => {}
                Self::DT_PLTRELSZ | Self::DT_SCE_PLTRELSZ => pltrelsz = Some(LE::read_u64(value)),
                Self::DT_PLTGOT | Self::DT_SCE_PLTGOT => pltgot = Some(LE::read_u64(value)),
                Self::DT_RELASZ | Self::DT_SCE_RELASZ => relasz = Some(LE::read_u64(value)),
                Self::DT_RELAENT | Self::DT_SCE_RELAENT => relaent = Some(LE::read_u64(value)),
                Self::DT_STRSZ | Self::DT_SCE_STRSZ => {}
                Self::DT_SYMENT | Self::DT_SCE_SYMENT => syment = Some(LE::read_u64(value)),
                Self::DT_INIT => {}
                Self::DT_FINI => {}
                Self::DT_SONAME => {}
                Self::DT_SYMBOLIC => {}
                Self::DT_PLTREL | Self::DT_SCE_PLTREL => pltrel = Some(LE::read_u64(value)),
                Self::DT_DEBUG => {}
                Self::DT_TEXTREL => {}
                Self::DT_INIT_ARRAY => {}
                Self::DT_FINI_ARRAY => {}
                Self::DT_INIT_ARRAYSZ => {}
                Self::DT_FINI_ARRAYSZ => {}
                Self::DT_FLAGS => {}
                Self::DT_PREINIT_ARRAY => {}
                Self::DT_PREINIT_ARRAYSZ => {}
                Self::DT_SCE_FINGERPRINT => fingerprint = Some(LE::read_u64(value)),
                Self::DT_SCE_FILENAME => filename = Some(LE::read_u64(value)),
                Self::DT_SCE_MODULE_INFO => {
                    module_info = Some(match parse_module_info(value) {
                        Some(v) if v.id == 0 => v,
                        _ => return Err(ParseError::InvalidModuleInfo),
                    });
                }
                Self::DT_SCE_NEEDED_MODULE => match parse_module_info(value) {
                    Some(v) if v.id != 0 => needed_modules.push(v),
                    _ => return Err(ParseError::InvalidNeededModule(index)),
                },
                Self::DT_SCE_MODULE_ATTR => {}
                Self::DT_SCE_EXPORT_LIB => match parse_library_info(value) {
                    Some(v) => exports.push(v),
                    None => return Err(ParseError::InvalidExport(index)),
                },
                Self::DT_SCE_IMPORT_LIB => match parse_library_info(value) {
                    Some(v) => imports.push(v),
                    None => return Err(ParseError::InvalidImport(index)),
                },
                Self::DT_SCE_EXPORT_LIB_ATTR => {}
                Self::DT_SCE_IMPORT_LIB_ATTR => {}
                Self::DT_SCE_HASH => hash = Some(LE::read_u64(value)),
                Self::DT_SCE_JMPREL => jmprel = Some(LE::read_u64(value)),
                Self::DT_SCE_RELA => rela = Some(LE::read_u64(value)),
                Self::DT_SCE_STRTAB => {}
                Self::DT_SCE_SYMTAB => symtab = Some(LE::read_u64(value)),
                Self::DT_SCE_HASHSZ => hashsz = Some(LE::read_u64(value)),
                Self::DT_SCE_SYMTABSZ => symtabsz = Some(LE::read_u64(value)),
                _ => return Err(ParseError::UnknownTag(tag)),
            }

            offset += 16;
            index += 1;
        }

        let parsed = Self {
            pltrelsz: pltrelsz.ok_or(ParseError::NoPltrelsz)?,
            pltgot: pltgot.ok_or(ParseError::NoPltgot)?,
            relasz: relasz.ok_or(ParseError::NoRelasz)?,
            relaent: relaent.ok_or(ParseError::NoRelaent)?,
            syment: syment.ok_or(ParseError::NoSyment)?,
            pltrel: pltrel.ok_or(ParseError::NoPltrel)?,
            fingerprint: fingerprint.ok_or(ParseError::NoFingerprint)?,
            filename: filename.ok_or(ParseError::NoFilename)?,
            module_info: module_info.ok_or(ParseError::NoModuleInfo)?,
            needed_modules,
            exports,
            imports,
            hash: hash.ok_or(ParseError::NoHash)?,
            jmprel: jmprel.ok_or(ParseError::NoJmprel)?,
            rela: rela.ok_or(ParseError::NoRela)?,
            symtab: symtab.ok_or(ParseError::NoSymtab)?,
            hashsz: hashsz.ok_or(ParseError::NoHashsz)?,
            symtabsz: symtabsz.ok_or(ParseError::NoSymtabsz)?,
        };

        // Check values.
        if parsed.relaent != 24 {
            // sizeof(Elf64_Rela)
            return Err(ParseError::InvalidRelaent);
        } else if parsed.syment != 24 {
            // sizeof(Elf64_Sym)
            return Err(ParseError::InvalidSyment);
        } else if parsed.pltrel != DynamicLinking::DT_RELA as _ {
            return Err(ParseError::InvalidPltrel);
        }

        Ok(parsed)
    }

    pub fn module_info(&self) -> &ModuleInfo {
        &self.module_info
    }

    pub fn needed_modules(&self) -> &[ModuleInfo] {
        self.needed_modules.as_ref()
    }

    pub fn exports(&self) -> &[LibraryInfo] {
        self.exports.as_ref()
    }

    pub fn imports(&self) -> &[LibraryInfo] {
        self.imports.as_ref()
    }
}

/// Contains information about the module.
pub struct ModuleInfo {
    id: u16,
    name: String,
    version_major: u8,
    version_minor: u8,
}

impl ModuleInfo {
    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn version_major(&self) -> u8 {
        self.version_major
    }

    pub fn version_minor(&self) -> u8 {
        self.version_minor
    }
}

/// Contains information about the library in the module.
pub struct LibraryInfo {
    id: u16,
    name: String,
    version: u16,
}

impl LibraryInfo {
    /// Gets the ID of this library.
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Gets the name of this library.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn version(&self) -> u16 {
        self.version
    }
}

/// Represents an error for [`DynamicLinking::parse()`].
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid data size")]
    InvalidDataSize,

    #[error("entry DT_PLTRELSZ or DT_SCE_PLTRELSZ does not exists")]
    NoPltrelsz,

    #[error("entry DT_PLTGOT or DT_SCE_PLTGOT does not exists")]
    NoPltgot,

    #[error("entry DT_RELASZ or DT_SCE_RELASZ does not exists")]
    NoRelasz,

    #[error("entry DT_RELAENT or DT_SCE_RELAENT does not exists")]
    NoRelaent,

    #[error("entry DT_STRSZ or DT_SCE_STRSZ does not exists")]
    NoStrsz,

    #[error("entry DT_SYMENT or DT_SCE_SYMENT does not exists")]
    NoSyment,

    #[error("entry DT_PLTREL or DT_SCE_PLTREL does not exists")]
    NoPltrel,

    #[error("entry DT_SCE_FINGERPRINT does not exists")]
    NoFingerprint,

    #[error("entry DT_SCE_FILENAME does not exists")]
    NoFilename,

    #[error("entry DT_SCE_MODULE_INFO does not exists")]
    NoModuleInfo,

    #[error("entry DT_SCE_HASH does not exists")]
    NoHash,

    #[error("entry DT_SCE_JMPREL does not exists")]
    NoJmprel,

    #[error("entry DT_SCE_RELA does not exists")]
    NoRela,

    #[error("entry DT_SCE_STRTAB does not exists")]
    NoStrtab,

    #[error("entry DT_SCE_SYMTAB does not exists")]
    NoSymtab,

    #[error("entry DT_SCE_HASHSZ does not exists")]
    NoHashsz,

    #[error("entry DT_SCE_SYMTABSZ does not exists")]
    NoSymtabsz,

    #[error("unknown tag {0:#018x}")]
    UnknownTag(i64),

    #[error("entry DT_RELAENT or DT_SCE_RELAENT has invalid value")]
    InvalidRelaent,

    #[error("entry DT_SYMENT or DT_SCE_SYMENT has invalid value")]
    InvalidSyment,

    #[error("entry DT_PLTREL or DT_SCE_PLTREL has value other than DT_RELA")]
    InvalidPltrel,

    #[error("entry DT_SCE_STRTAB has invalid value")]
    InvalidStrtab,

    #[error("entry DT_SCE_MODULE_INFO has invalid value")]
    InvalidModuleInfo,

    #[error("entry {0} is not a valid DT_SCE_NEEDED_MODULE")]
    InvalidNeededModule(usize),

    #[error("entry {0} is not a valid DT_SCE_EXPORT_LIB")]
    InvalidExport(usize),

    #[error("entry {0} is not a valid DT_SCE_IMPORT_LIB")]
    InvalidImport(usize),
}
