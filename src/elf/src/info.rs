use crate::{
    DynamicEntries, DynamicTag, LibraryFlags, LibraryInfo, ModuleInfo, Relocations, Symbols,
};
use byteorder::{ByteOrder, LE};
use thiserror::Error;

/// An object that is initialized by `acquire_per_file_info_obj`.
#[derive(Debug)]
pub struct FileInfo {
    data: Vec<u8>,
    comment: Vec<u8>,
    dynoff: usize,
    dynsize: usize,
    pltrelsz: usize,
    relasz: usize,
    strsz: usize,
    jmprel: usize,
    rela: usize,
    strtab: usize,
    symtab: usize,
    symtabsz: usize,
    buckets: Vec<u32>,
    chains: Vec<u32>,
}

impl FileInfo {
    pub(super) fn parse(
        data: Vec<u8>,
        comment: Vec<u8>,
        dynoff: usize,
        dynsize: usize,
    ) -> Result<Self, FileInfoError> {
        // Parse dynamic.
        let mut pltrelsz: Option<u64> = None;
        let mut relasz: Option<u64> = None;
        let mut relaent = false;
        let mut strsz: Option<u64> = None;
        let mut syment = false;
        let mut pltrel = false;
        let mut fingerprint = false;
        let mut filename = false;
        let mut module_info = false;
        let mut hash: Option<u64> = None;
        let mut pltgot = false;
        let mut jmprel: Option<u64> = None;
        let mut rela: Option<u64> = None;
        let mut strtab: Option<u64> = None;
        let mut symtab: Option<u64> = None;
        let mut hashsz: Option<u64> = None;
        let mut symtabsz: Option<u64> = None;

        // Let it panic if the dynamic size is not correct because the PS4 also does not check for
        // this.
        for (tag, value) in DynamicEntries::new(&data[dynoff..(dynoff + dynsize)]) {
            match tag {
                DynamicTag::DT_NULL => break,
                DynamicTag::DT_NEEDED => {}
                DynamicTag::DT_PLTRELSZ | DynamicTag::DT_SCE_PLTRELSZ => {
                    pltrelsz = Some(u64::from_le_bytes(value))
                }
                DynamicTag::DT_PLTGOT
                | DynamicTag::DT_RPATH
                | DynamicTag::DT_BIND_NOW
                | DynamicTag::DT_RUNPATH
                | DynamicTag::DT_ENCODING
                | DynamicTag::DT_SCE_UNK2
                | DynamicTag::DT_SCE_UNK3
                | DynamicTag::DT_SCE_UNK4
                | DynamicTag::DT_SCE_UNK5
                | DynamicTag::DT_SCE_UNK6
                | DynamicTag::DT_SCE_UNK7
                | DynamicTag::DT_SCE_UNK8
                | DynamicTag::DT_SCE_UNK9
                | DynamicTag::DT_SCE_UNK10
                | DynamicTag::DT_SCE_UNK11
                | DynamicTag::DT_SCE_UNK12
                | DynamicTag::DT_SCE_UNK13
                | DynamicTag::DT_SCE_UNK14
                | DynamicTag::DT_SCE_STUB_MODULE_NAME
                | DynamicTag::DT_SCE_UNK16
                | DynamicTag::DT_SCE_STUB_MODULE_VERSION
                | DynamicTag::DT_SCE_UNK18
                | DynamicTag::DT_SCE_STUB_LIBRARY_NAME
                | DynamicTag::DT_SCE_UNK20
                | DynamicTag::DT_SCE_STUB_LIBRARY_VERSION
                | DynamicTag::DT_SCE_UNK22
                | DynamicTag::DT_SCE_UNK23
                | DynamicTag::DT_SCE_UNK24
                | DynamicTag::DT_SCE_UNK25
                | DynamicTag::DT_SCE_UNK26
                | DynamicTag::DT_SCE_UNK27
                | DynamicTag::DT_SCE_UNK28
                | DynamicTag::DT_SCE_UNK29
                | DynamicTag::DT_SCE_UNK30
                | DynamicTag::DT_SCE_UNK31
                | DynamicTag::DT_SCE_UNK32
                | DynamicTag::DT_SCE_UNK33
                | DynamicTag::DT_SCE_UNK34
                | DynamicTag::DT_SCE_UNK35 => {
                    return Err(FileInfoError::UnsupportedTag(tag));
                }
                DynamicTag::DT_HASH
                | DynamicTag::DT_STRTAB
                | DynamicTag::DT_SYMTAB
                | DynamicTag::DT_RELA
                | DynamicTag::DT_JMPREL
                | DynamicTag::DT_REL
                | DynamicTag::DT_RELSZ
                | DynamicTag::DT_RELENT => {
                    return Err(FileInfoError::OrbisUnsupported(tag));
                }
                DynamicTag::DT_RELASZ | DynamicTag::DT_SCE_RELASZ => {
                    relasz = Some(u64::from_le_bytes(value))
                }
                DynamicTag::DT_RELAENT | DynamicTag::DT_SCE_RELAENT => {
                    relaent = if u64::from_le_bytes(value) == 24 {
                        true
                    } else {
                        return Err(FileInfoError::InvalidRelaent);
                    }
                }
                DynamicTag::DT_STRSZ | DynamicTag::DT_SCE_STRSZ => {
                    strsz = Some(u64::from_le_bytes(value))
                }
                DynamicTag::DT_SYMENT | DynamicTag::DT_SCE_SYMENT => {
                    syment = if u64::from_le_bytes(value) == 24 {
                        true
                    } else {
                        return Err(FileInfoError::InvalidSyment);
                    }
                }
                DynamicTag::DT_INIT => {}
                DynamicTag::DT_FINI => {}
                DynamicTag::DT_SONAME => {}
                DynamicTag::DT_SYMBOLIC => {}
                DynamicTag::DT_PLTREL | DynamicTag::DT_SCE_PLTREL => {
                    pltrel = if u64::from_le_bytes(value) == 7 {
                        true
                    } else {
                        return Err(FileInfoError::InvalidPltrel);
                    }
                }
                DynamicTag::DT_DEBUG => {}
                DynamicTag::DT_TEXTREL => {}
                DynamicTag::DT_INIT_ARRAY => {}
                DynamicTag::DT_FINI_ARRAY => {}
                DynamicTag::DT_INIT_ARRAYSZ => {}
                DynamicTag::DT_FINI_ARRAYSZ => {}
                DynamicTag::DT_FLAGS => {}
                DynamicTag::DT_PREINIT_ARRAY => {}
                DynamicTag::DT_PREINIT_ARRAYSZ => {}
                DynamicTag::DT_SCE_UNK1 => {}
                DynamicTag::DT_SCE_FINGERPRINT => fingerprint = true,
                DynamicTag::DT_SCE_ORIGINAL_FILENAME => filename = true,
                DynamicTag::DT_SCE_MODULE_INFO => module_info = true,
                DynamicTag::DT_SCE_NEEDED_MODULE => {}
                DynamicTag::DT_SCE_MODULE_ATTR => {}
                DynamicTag::DT_SCE_EXPORT_LIB => {}
                DynamicTag::DT_SCE_IMPORT_LIB => {}
                DynamicTag::DT_SCE_EXPORT_LIB_ATTR => {}
                DynamicTag::DT_SCE_IMPORT_LIB_ATTR => {}
                DynamicTag::DT_SCE_HASH => hash = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_PLTGOT => pltgot = true,
                DynamicTag::DT_SCE_JMPREL => jmprel = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_RELA => rela = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_STRTAB => strtab = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_SYMTAB => symtab = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_HASHSZ => hashsz = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_SYMTABSZ => symtabsz = Some(u64::from_le_bytes(value)),
                DynamicTag::DT_SCE_UNK36 => {}
                DynamicTag::DT_SCE_UNK37 => {}
                v => return Err(FileInfoError::UnknownTag(v)),
            }
        }

        // Check required tags.
        let pltrelsz = pltrelsz.ok_or(FileInfoError::NoPltrelsz)?;
        let relasz = relasz.ok_or(FileInfoError::NoRelasz)?;
        let strsz = strsz.ok_or(FileInfoError::NoStrsz)?;
        let hash = hash.ok_or(FileInfoError::NoHash)?;
        let jmprel = jmprel.ok_or(FileInfoError::NoJmprel)?;
        let rela = rela.ok_or(FileInfoError::NoRela)?;
        let strtab = strtab.ok_or(FileInfoError::NoStrtab)?;
        let symtab = symtab.ok_or(FileInfoError::NoSymtab)?;
        let hashsz = hashsz.ok_or(FileInfoError::NoHashsz)?;
        let symtabsz = symtabsz.ok_or(FileInfoError::NoSymtabsz)?;

        if !relaent {
            return Err(FileInfoError::NoRelaent);
        } else if !syment {
            return Err(FileInfoError::NoSyment);
        } else if !pltrel {
            return Err(FileInfoError::NoPltrel);
        } else if !fingerprint {
            return Err(FileInfoError::NoFingerprint);
        } else if !filename {
            return Err(FileInfoError::NoFilename);
        } else if !module_info {
            return Err(FileInfoError::NoModuleInfo);
        } else if !pltgot {
            return Err(FileInfoError::NoPltgot);
        }

        // Read hash table.
        let hash: usize = hash.try_into().unwrap();
        let hashsz: usize = hashsz.try_into().unwrap();
        let hash = &data[hash..(hash + hashsz)];
        let nbuckets: usize = LE::read_u32(hash).try_into().unwrap();
        let nchains: usize = LE::read_u32(&hash[4..]).try_into().unwrap();
        let mut buckets = vec![0; nbuckets];
        let mut chains = vec![0; nchains];
        let (first, last) = hash.split_at(8 + nbuckets * 4);

        LE::read_u32_into(&first[8..], &mut buckets);
        LE::read_u32_into(last, &mut chains);

        // TODO: Check acquire_per_file_info_obj to see what we have missing here.
        Ok(Self {
            data,
            comment,
            dynoff,
            dynsize,
            pltrelsz: pltrelsz.try_into().unwrap(),
            relasz: relasz.try_into().unwrap(),
            strsz: strsz.try_into().unwrap(),
            jmprel: jmprel.try_into().unwrap(),
            rela: rela.try_into().unwrap(),
            strtab: strtab.try_into().unwrap(),
            symtab: symtab.try_into().unwrap(),
            symtabsz: symtabsz.try_into().unwrap(),
            buckets,
            chains,
        })
    }

    pub fn comment(&self) -> &[u8] {
        self.comment.as_ref()
    }

    pub fn dynamic(&self) -> DynamicEntries<'_> {
        DynamicEntries::new(&self.data[self.dynoff..(self.dynoff + self.dynsize)])
    }

    pub fn reloc_count(&self) -> usize {
        self.relasz / 24
    }

    pub fn relocs(&self) -> Relocations<'_> {
        Relocations::new(&self.data[self.rela..(self.rela + self.relasz)])
    }

    pub fn plt_count(&self) -> usize {
        self.pltrelsz / 24
    }

    pub fn plt_relocs(&self) -> Relocations<'_> {
        Relocations::new(&self.data[self.jmprel..(self.jmprel + self.pltrelsz)])
    }

    pub fn symbol_count(&self) -> usize {
        self.symtabsz / 24
    }

    pub fn symbols(&self) -> Symbols<'_> {
        Symbols::new(&self.data[self.symtab..(self.symtab + self.symtabsz)], self)
    }

    pub fn buckets(&self) -> &[u32] {
        self.buckets.as_ref()
    }

    pub fn chains(&self) -> &[u32] {
        self.chains.as_ref()
    }

    pub fn read_module(&self, data: [u8; 8]) -> Result<ModuleInfo, ReadModuleError> {
        // Load data.
        let name = LE::read_u32(&data);
        let id = LE::read_u16(&data[6..]);

        // Lookup name.
        let name = match self.read_str(name.try_into().unwrap()) {
            Ok(v) => v.to_owned(),
            Err(e) => return Err(ReadModuleError::InvalidNameOffset(name, e)),
        };

        Ok(ModuleInfo::new(id, name))
    }

    pub fn read_library(&self, data: [u8; 8]) -> Result<LibraryInfo, ReadLibraryError> {
        // Load data.
        let name = LE::read_u32(&data);
        let id = LE::read_u16(&data[6..]);

        // Lookup name.
        let name = match self.read_str(name.try_into().unwrap()) {
            Ok(v) => v.to_owned(),
            Err(e) => return Err(ReadLibraryError::InvalidNameOffset(name, e)),
        };

        Ok(LibraryInfo::new(id, name, LibraryFlags::empty()))
    }

    pub fn read_str(&self, offset: usize) -> Result<&str, StringTableError> {
        // Get raw string.
        let tab = &self.data[self.strtab..(self.strtab + self.strsz)];
        let raw = match tab.get(offset..) {
            Some(v) if !v.is_empty() => v,
            _ => return Err(StringTableError::InvalidOffset),
        };

        // Find a NULL-terminated.
        let raw = match raw.iter().position(|&b| b == 0) {
            Some(i) => &raw[..i],
            None => return Err(StringTableError::NotCString),
        };

        // Get Rust string.
        std::str::from_utf8(raw).map_err(|_| StringTableError::NotUtf8)
    }
}

/// Represents an error for file info parsing.
#[derive(Debug, Error)]
pub enum FileInfoError {
    #[error("unknown tag {0}")]
    UnknownTag(DynamicTag),

    #[error("tag {0} is not supported")]
    UnsupportedTag(DynamicTag),

    #[error("Orbis object file does not support tag {0}")]
    OrbisUnsupported(DynamicTag),

    #[error("no DT_PLTRELSZ or DT_SCE_PLTRELSZ")]
    NoPltrelsz,

    #[error("no DT_RELASZ or DT_SCE_RELASZ")]
    NoRelasz,

    #[error("DT_RELAENT or DT_SCE_RELAENT has invalid value")]
    InvalidRelaent,

    #[error("no DT_RELAENT or DT_SCE_RELAENT")]
    NoRelaent,

    #[error("no DT_STRSZ or DT_SCE_STRSZ")]
    NoStrsz,

    #[error("DT_SYMENT or DT_SCE_SYMENT has invalid value")]
    InvalidSyment,

    #[error("no DT_SYMENT or DT_SCE_SYMENT")]
    NoSyment,

    #[error("DT_PLTREL or DT_SCE_PLTREL has invalid value")]
    InvalidPltrel,

    #[error("no DT_PLTREL or DT_SCE_PLTREL")]
    NoPltrel,

    #[error("no DT_SCE_FINGERPRINT")]
    NoFingerprint,

    #[error("no DT_SCE_ORIGINAL_FILENAME")]
    NoFilename,

    #[error("no DT_SCE_MODULE_INFO")]
    NoModuleInfo,

    #[error("no DT_SCE_HASH")]
    NoHash,

    #[error("no DT_SCE_PLTGOT")]
    NoPltgot,

    #[error("no DT_SCE_JMPREL")]
    NoJmprel,

    #[error("no DT_SCE_RELA")]
    NoRela,

    #[error("no DT_SCE_STRTAB")]
    NoStrtab,

    #[error("no DT_SCE_SYMTAB")]
    NoSymtab,

    #[error("no DT_SCE_HASHSZ")]
    NoHashsz,

    #[error("no DT_SCE_SYMTABSZ")]
    NoSymtabsz,
}

/// Represents an error when reading [`ModuleInfo`] is failed.
#[derive(Debug, Error)]
pub enum ReadModuleError {
    #[error("name offset {0} is not valid")]
    InvalidNameOffset(u32, #[source] StringTableError),
}

/// Represents an error when reading [`LibraryInfo`] is failed.
#[derive(Debug, Error)]
pub enum ReadLibraryError {
    #[error("name offset {0} is not valid")]
    InvalidNameOffset(u32, #[source] StringTableError),
}

/// Represents an error when string table lookup is failed.
#[derive(Debug, Error)]
pub enum StringTableError {
    #[error("the offset is not a valid offset in the string table")]
    InvalidOffset,

    #[error("the offset is not a C string")]
    NotCString,

    #[error("the offset is not a UTF-8 string")]
    NotUtf8,
}
