use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Index;
use std::slice::SliceIndex;
use thiserror::Error;

/// Contains data required for dynamic linking.
pub struct DynamicLinking {
    needed: Vec<String>,
    pltrelsz: usize,
    pltgot: u64,
    relasz: usize,
    relaent: usize,
    pltrel: u64,
    fingerprint: u64,
    flags: Option<ModuleFlags>,
    filename: u64,
    module_info: ModuleInfo,
    dependencies: HashMap<u16, ModuleInfo>,
    libraries: HashMap<u16, LibraryInfo>,
    symbols: Vec<SymbolInfo>,
    symbol_lookup_table: Vec<u32>,
    jmprel: usize,
    rela: usize,
    data: DynlibData,
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

    pub(super) fn parse(data: Vec<u8>, dynlib: Vec<u8>) -> Result<Self, ParseError> {
        // Simple check to see if data valid.
        if data.len() % 16 != 0 {
            return Err(ParseError::InvalidDataSize);
        }

        // Find the offset of data tables.
        let mut strtab: Option<u64> = None;
        let mut strsz: Option<u64> = None;

        for data in data.chunks_exact(16) {
            // Read fields.
            let tag = LE::read_i64(&data);
            let value = &data[8..];

            // Parse entry.
            match tag {
                Self::DT_SCE_STRTAB => strtab = Some(LE::read_u64(value)),
                Self::DT_STRSZ | Self::DT_SCE_STRSZ => strsz = Some(LE::read_u64(value)),
                _ => {}
            }
        }

        // Check string table.
        let strtab = strtab.ok_or(ParseError::NoStrtab)? as usize;
        let strsz = strsz.ok_or(ParseError::NoStrsz)? as usize;

        if strtab + strsz > dynlib.len() {
            return Err(ParseError::InvalidStrtab);
        }

        let dynlib = DynlibData {
            data: dynlib,
            strtab,
            strsz,
        };

        // Get data tables.
        let parse_module_info = |value: &[u8]| -> Option<ModuleInfo> {
            let name = LE::read_u32(&value) as usize;
            let version_minor = value[4];
            let version_major = value[5];
            let id = LE::read_u16(&value[6..]);

            Some(ModuleInfo {
                id,
                name: dynlib.str(name)?,
                version_major,
                version_minor,
            })
        };

        // Parse entries.
        let mut needed: Vec<String> = Vec::new();
        let mut pltrelsz: Option<u64> = None;
        let mut pltgot: Option<u64> = None;
        let mut relasz: Option<u64> = None;
        let mut relaent: Option<u64> = None;
        let mut syment: Option<u64> = None;
        let mut pltrel: Option<u64> = None;
        let mut fingerprint: Option<u64> = None;
        let mut flags: Option<ModuleFlags> = None;
        let mut filename: Option<u64> = None;
        let mut module_info: Option<ModuleInfo> = None;
        let mut dependencies: HashMap<u16, ModuleInfo> = HashMap::new();
        let mut libraries: HashMap<u16, LibraryInfo> = HashMap::new();
        let mut hash: Option<u64> = None;
        let mut jmprel: Option<u64> = None;
        let mut rela: Option<u64> = None;
        let mut symtab: Option<u64> = None;
        let mut hashsz: Option<u64> = None;
        let mut symtabsz: Option<u64> = None;

        for (index, data) in data.chunks_exact(16).enumerate() {
            use std::collections::hash_map::Entry;

            // Read fields.
            let tag = LE::read_i64(&data);
            let value = &data[8..];

            // Parse entry.
            match tag {
                Self::DT_NULL => break,
                Self::DT_NEEDED => match dynlib.str(LE::read_u64(value) as usize) {
                    Some(v) => needed.push(v),
                    None => return Err(ParseError::InvalidNeeded(index)),
                },
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
                Self::DT_FLAGS => flags = Some(ModuleFlags::from_bits_retain(LE::read_u64(value))),
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
                    Some(v) if v.id != 0 => match dependencies.entry(v.id) {
                        Entry::Occupied(_) => {
                            return Err(ParseError::DuplicatedNeededModule(index));
                        }
                        Entry::Vacant(e) => {
                            e.insert(v);
                        }
                    },
                    _ => return Err(ParseError::InvalidNeededModule(index)),
                },
                Self::DT_SCE_MODULE_ATTR => {}
                Self::DT_SCE_EXPORT_LIB | Self::DT_SCE_IMPORT_LIB => {
                    // Parse the value.
                    let name = LE::read_u32(value) as usize;
                    let version = LE::read_u16(&value[4..]);
                    let id = LE::read_u16(&value[6..]);
                    let is_export = tag == Self::DT_SCE_EXPORT_LIB;
                    let info = LibraryInfo {
                        id,
                        name: match dynlib.str(name) {
                            Some(v) => v,
                            None => {
                                return Err(if is_export {
                                    ParseError::InvalidExport(index)
                                } else {
                                    ParseError::InvalidImport(index)
                                })
                            }
                        },
                        version,
                        is_export,
                    };

                    // Store the info.
                    match libraries.entry(id) {
                        Entry::Occupied(_) => return Err(ParseError::DuplicatedLibrary(index)),
                        Entry::Vacant(e) => e.insert(info),
                    };
                }
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
        }

        // Check symbol hash table.
        let hash = hash.ok_or(ParseError::NoHash)? as usize;
        let hashsz = hashsz.ok_or(ParseError::NoHashsz)? as usize;

        if hashsz < 8 || hashsz % 4 != 0 {
            return Err(ParseError::InvalidHashsz);
        }

        // Parse symbol hash table.
        let mut symbol_lookup_table: Vec<u32> = vec![0u32; hashsz / 4];

        match dynlib.get(hash..(hash + hashsz)) {
            Some(v) => LE::read_u32_into(v, &mut symbol_lookup_table),
            None => return Err(ParseError::InvalidHash),
        }

        // Check size of symbol entry.
        let syment = match syment {
            Some(v) => {
                if v != 24 {
                    // sizeof(Elf64_Sym)
                    return Err(ParseError::InvalidSyment);
                }

                v as usize
            }
            None => return Err(ParseError::NoSyment),
        };

        // Parse symbol table.
        let symbols = match (symtab, symtabsz) {
            (Some(offset), Some(size)) => {
                let offset = offset as usize;
                let size = size as usize;

                // Check if size valid.
                if size % syment != 0 {
                    return Err(ParseError::InvalidSymtabsz);
                }

                // Get the table.
                let table = match dynlib.get(offset..(offset + size)) {
                    Some(v) => v,
                    None => return Err(ParseError::InvalidSymtab),
                };

                // Parse the table.
                let mut symbols: Vec<SymbolInfo> = Vec::with_capacity(size / syment);

                for (i, e) in table.chunks(syment).enumerate() {
                    let name = LE::read_u32(e) as usize;
                    let info = e[4];
                    let value = LE::read_u64(&e[8..]) as usize;

                    symbols.push(SymbolInfo {
                        name: match dynlib.str(name) {
                            Some(v) => v,
                            None => return Err(ParseError::InvalidSymbol(i)),
                        },
                        info,
                        value,
                    })
                }

                symbols
            }
            (None, _) => return Err(ParseError::NoSymtab),
            (_, None) => return Err(ParseError::NoSymtabsz),
        };

        let parsed = Self {
            needed,
            pltrelsz: pltrelsz.ok_or(ParseError::NoPltrelsz)? as usize,
            pltgot: pltgot.ok_or(ParseError::NoPltgot)?,
            relasz: relasz.ok_or(ParseError::NoRelasz)? as usize,
            relaent: relaent.ok_or(ParseError::NoRelaent)? as usize,
            pltrel: pltrel.ok_or(ParseError::NoPltrel)?,
            fingerprint: fingerprint.ok_or(ParseError::NoFingerprint)?,
            flags: match flags {
                Some(v) => {
                    if v.is_empty() {
                        None
                    } else {
                        Some(v)
                    }
                }
                None => return Err(ParseError::NoFlags),
            },
            filename: filename.ok_or(ParseError::NoFilename)?,
            module_info: module_info.ok_or(ParseError::NoModuleInfo)?,
            dependencies,
            libraries,
            symbols,
            symbol_lookup_table,
            jmprel: jmprel.ok_or(ParseError::NoJmprel)? as usize,
            rela: rela.ok_or(ParseError::NoRela)? as usize,
            data: dynlib,
        };

        // Check values.
        if parsed.relaent != 24 {
            // sizeof(Elf64_Rela)
            return Err(ParseError::InvalidRelaent);
        } else if parsed.pltrel != DynamicLinking::DT_RELA as _ {
            return Err(ParseError::InvalidPltrel);
        }

        Ok(parsed)
    }

    pub fn needed(&self) -> &[String] {
        self.needed.as_ref()
    }

    pub fn flags(&self) -> Option<ModuleFlags> {
        self.flags
    }

    pub fn module_info(&self) -> &ModuleInfo {
        &self.module_info
    }

    pub fn dependencies(&self) -> &HashMap<u16, ModuleInfo> {
        &self.dependencies
    }

    pub fn libraries(&self) -> &HashMap<u16, LibraryInfo> {
        &self.libraries
    }

    pub fn symbols(&self) -> &[SymbolInfo] {
        self.symbols.as_ref()
    }

    pub fn relocation_entries(&self) -> RelocationEntries<'_> {
        RelocationEntries {
            relaent: self.relaent,
            next: &self.data[self.rela..(self.rela + self.relasz)],
        }
    }

    pub fn plt_relocation(&self) -> RelocationEntries<'_> {
        RelocationEntries {
            relaent: self.relaent,
            next: &self.data[self.jmprel..(self.jmprel + self.pltrelsz)],
        }
    }

    pub fn lookup_symbol(&self, hash: u32, name: &str) -> Option<&SymbolInfo> {
        // Get hash table.
        let bucket_count = self.symbol_lookup_table[0] as usize;
        let chain_count = self.symbol_lookup_table[1] as usize;
        let buckets = &self.symbol_lookup_table[2..];
        let chains = &buckets[bucket_count..];

        // Lookup.
        let mut index = buckets[hash as usize % bucket_count] as usize;

        while index != 0 {
            if index >= chain_count {
                return None;
            }

            // Parse symbol name.
            let sym = &self.symbols[index];
            let (sym_name, lib_id, mod_id) = match sym.decode_name() {
                Some(v) => v,
                None => {
                    index = chains[index] as usize;
                    continue;
                }
            };

            if mod_id != 0 {
                index = chains[index] as usize;
                continue;
            }

            // Get target library.
            let lib = match self.libraries.get(&lib_id) {
                Some(v) => v,
                None => panic!("Unexpected library ID: {lib_id}"),
            };

            if !lib.is_export() {
                panic!("The target library is not an exported library.");
            }

            // Check if matched.
            if name == format!("{sym_name}#{}#{}", lib.name, self.module_info.name) {
                return Some(sym);
            }

            index = chains[index] as usize;
        }

        None
    }
}

bitflags! {
    /// Contains flags for a module.
    #[derive(Clone, Copy)]
    pub struct ModuleFlags: u64 {
        const DF_SYMBOLIC = 0x02; // Not used in PS4.
        const DF_TEXTREL = 0x04;
        const DF_BIND_NOW = 0x08; // Not used in PS4.
    }
}

impl Display for ModuleFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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
    is_export: bool,
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

    pub fn is_export(&self) -> bool {
        self.is_export
    }
}

/// Contains information about a symbol in the SELF.
pub struct SymbolInfo {
    name: String,
    info: u8,
    value: usize,
}

impl SymbolInfo {
    /// Local symbol, not visible outside obj file containing def.
    pub const STB_LOCAL: u8 = 0;

    /// Global symbol, visible to all object files being combined.
    pub const STB_GLOBAL: u8 = 1;

    /// Weak symbol, like global but lower-precedence.
    pub const STB_WEAK: u8 = 2;

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn binding(&self) -> u8 {
        self.info >> 4
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

/// An iterator over the relocation entry of the SELF.
pub struct RelocationEntries<'a> {
    relaent: usize,
    next: &'a [u8],
}

impl<'a> Iterator for RelocationEntries<'a> {
    type Item = RelocationInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_empty() {
            return None;
        }

        // FIXME: Handle invalid data instead of panic.
        let offset = LE::read_u64(self.next) as usize;
        let info = LE::read_u64(&self.next[8..]);
        let addend = LE::read_u64(&self.next[16..]) as usize;

        self.next = &self.next[self.relaent..];

        Some(RelocationInfo {
            offset,
            info,
            addend,
        })
    }
}

/// Contains information required to relocate a specific address.
pub struct RelocationInfo {
    offset: usize,
    info: u64,
    addend: usize,
}

impl RelocationInfo {
    pub const R_X86_64_NONE: u32 = 0;
    pub const R_X86_64_64: u32 = 1;
    pub const R_X86_64_PC32: u32 = 2;
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    pub const R_X86_64_RELATIVE: u32 = 8;
    pub const R_X86_64_DTPMOD64: u32 = 16;
    pub const R_X86_64_DTPOFF64: u32 = 17;
    pub const R_X86_64_TPOFF64: u32 = 18;
    pub const R_X86_64_DTPOFF32: u32 = 21;
    pub const R_X86_64_TPOFF32: u32 = 23;

    pub fn ty(&self) -> u32 {
        (self.info & 0x00000000ffffffff) as u32
    }

    pub fn symbol(&self) -> usize {
        (self.info >> 32) as usize
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn addend(&self) -> usize {
        self.addend
    }
}

/// Encapsulate a PT_SCE_DYNLIBDATA.
struct DynlibData {
    data: Vec<u8>,
    strtab: usize,
    strsz: usize,
}

impl DynlibData {
    fn str(&self, offset: usize) -> Option<String> {
        let raw = match self.data[self.strtab..(self.strtab + self.strsz)].get(offset..) {
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
    }

    fn get<I: SliceIndex<[u8]>>(&self, index: I) -> Option<&I::Output> {
        self.data.get(index)
    }
}

impl<I> Index<I> for DynlibData
where
    I: SliceIndex<[u8]>,
{
    type Output = <I as SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.data.index(index)
    }
}

/// Represents an error for [`DynamicLinking::parse()`].
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid data size")]
    InvalidDataSize,

    #[error("entry {0} is not a valid DT_NEEDED")]
    InvalidNeeded(usize),

    #[error("entry DT_PLTRELSZ or DT_SCE_PLTRELSZ does not exist")]
    NoPltrelsz,

    #[error("entry DT_PLTGOT or DT_SCE_PLTGOT does not exist")]
    NoPltgot,

    #[error("entry DT_RELASZ or DT_SCE_RELASZ does not exist")]
    NoRelasz,

    #[error("entry DT_RELAENT or DT_SCE_RELAENT does not exist")]
    NoRelaent,

    #[error("entry DT_STRSZ or DT_SCE_STRSZ does not exist")]
    NoStrsz,

    #[error("entry DT_SYMENT or DT_SCE_SYMENT does not exist")]
    NoSyment,

    #[error("entry DT_PLTREL or DT_SCE_PLTREL does not exist")]
    NoPltrel,

    #[error("entry DT_SCE_FINGERPRINT does not exist")]
    NoFingerprint,

    #[error("entry DT_FLAGS does not exist")]
    NoFlags,

    #[error("entry DT_SCE_FILENAME does not exist")]
    NoFilename,

    #[error("entry DT_SCE_MODULE_INFO does not exist")]
    NoModuleInfo,

    #[error("entry DT_SCE_HASH does not exist")]
    NoHash,

    #[error("entry DT_SCE_HASH has invalid value")]
    InvalidHash,

    #[error("entry DT_SCE_JMPREL does not exist")]
    NoJmprel,

    #[error("entry DT_SCE_RELA does not exist")]
    NoRela,

    #[error("entry DT_SCE_STRTAB does not exist")]
    NoStrtab,

    #[error("entry DT_SCE_SYMTAB does not exist")]
    NoSymtab,

    #[error("entry DT_SCE_SYMTAB has invalid value")]
    InvalidSymtab,

    #[error("entry DT_SCE_HASHSZ does not exist")]
    NoHashsz,

    #[error("entry DT_SCE_HASHSZ has invalid value")]
    InvalidHashsz,

    #[error("entry DT_SCE_SYMTABSZ does not exist")]
    NoSymtabsz,

    #[error("entry DT_SCE_SYMTABSZ has invalid value")]
    InvalidSymtabsz,

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

    #[error("duplicated needed module on entry {0}")]
    DuplicatedNeededModule(usize),

    #[error("entry {0} is not a valid DT_SCE_EXPORT_LIB")]
    InvalidExport(usize),

    #[error("entry {0} is not a valid DT_SCE_IMPORT_LIB")]
    InvalidImport(usize),

    #[error("duplicated library on entry {0}")]
    DuplicatedLibrary(usize),

    #[error("Symbol entry {0} is not valid")]
    InvalidSymbol(usize),
}
