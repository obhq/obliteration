use byteorder::{ByteOrder, LE};
use thiserror::Error;

/// An object that is initialized by `acquire_per_file_info_obj`.
pub struct FileInfo {
    data: Vec<u8>,
    comment: Vec<u8>,
}

impl FileInfo {
    pub const DT_NULL: i64 = 0;
    pub const DT_NEEDED: i64 = 1;
    pub const DT_PLTRELSZ: i64 = 2;
    pub const DT_PLTGOT: i64 = 3;
    pub const DT_HASH: i64 = 4;
    pub const DT_STRTAB: i64 = 5;
    pub const DT_SYMTAB: i64 = 6;
    pub const DT_RELA: i64 = 7;
    pub const DT_RELASZ: i64 = 8;
    pub const DT_RELAENT: i64 = 9;
    pub const DT_STRSZ: i64 = 10;
    pub const DT_SYMENT: i64 = 11;
    pub const DT_INIT: i64 = 12;
    pub const DT_FINI: i64 = 13;
    pub const DT_SONAME: i64 = 14;
    pub const DT_RPATH: i64 = 15;
    pub const DT_SYMBOLIC: i64 = 16;
    pub const DT_REL: i64 = 17;
    pub const DT_RELSZ: i64 = 18;
    pub const DT_RELENT: i64 = 19;
    pub const DT_PLTREL: i64 = 20;
    pub const DT_DEBUG: i64 = 21;
    pub const DT_TEXTREL: i64 = 22;
    pub const DT_JMPREL: i64 = 23;
    pub const DT_BIND_NOW: i64 = 24;
    pub const DT_INIT_ARRAY: i64 = 25;
    pub const DT_FINI_ARRAY: i64 = 26;
    pub const DT_INIT_ARRAYSZ: i64 = 27;
    pub const DT_FINI_ARRAYSZ: i64 = 28;
    pub const DT_RUNPATH: i64 = 29;
    pub const DT_FLAGS: i64 = 30;
    pub const DT_ENCODING: i64 = 31;
    pub const DT_PREINIT_ARRAY: i64 = 32;
    pub const DT_PREINIT_ARRAYSZ: i64 = 33;
    pub const DT_SCE_UNK1: i64 = 0x60000005;
    pub const DT_SCE_FINGERPRINT: i64 = 0x61000007;
    pub const DT_SCE_UNK2: i64 = 0x61000008;
    pub const DT_SCE_UNK3: i64 = 0x6100000a;
    pub const DT_SCE_UNK4: i64 = 0x6100000b;
    pub const DT_SCE_UNK5: i64 = 0x6100000c;
    pub const DT_SCE_UNK6: i64 = 0x6100000e;
    pub const DT_SCE_FILENAME: i64 = 0x61000009;
    pub const DT_SCE_MODULE_INFO: i64 = 0x6100000d;
    pub const DT_SCE_NEEDED_MODULE: i64 = 0x6100000f;
    pub const DT_SCE_UNK7: i64 = 0x61000010;
    pub const DT_SCE_MODULE_ATTR: i64 = 0x61000011;
    pub const DT_SCE_UNK8: i64 = 0x61000012;
    pub const DT_SCE_EXPORT_LIB: i64 = 0x61000013;
    pub const DT_SCE_UNK9: i64 = 0x61000014;
    pub const DT_SCE_IMPORT_LIB: i64 = 0x61000015;
    pub const DT_SCE_UNK10: i64 = 0x61000016;
    pub const DT_SCE_EXPORT_LIB_ATTR: i64 = 0x61000017;
    pub const DT_SCE_UNK11: i64 = 0x61000018;
    pub const DT_SCE_IMPORT_LIB_ATTR: i64 = 0x61000019;
    pub const DT_SCE_UNK12: i64 = 0x6100001a;
    pub const DT_SCE_UNK13: i64 = 0x6100001b;
    pub const DT_SCE_UNK14: i64 = 0x6100001c;
    pub const DT_SCE_UNK15: i64 = 0x6100001d;
    pub const DT_SCE_UNK16: i64 = 0x6100001e;
    pub const DT_SCE_UNK17: i64 = 0x6100001f;
    pub const DT_SCE_UNK18: i64 = 0x61000020;
    pub const DT_SCE_UNK19: i64 = 0x61000021;
    pub const DT_SCE_UNK20: i64 = 0x61000022;
    pub const DT_SCE_UNK21: i64 = 0x61000023;
    pub const DT_SCE_UNK22: i64 = 0x61000024;
    pub const DT_SCE_HASH: i64 = 0x61000025;
    pub const DT_SCE_UNK23: i64 = 0x61000026;
    pub const DT_SCE_PLTGOT: i64 = 0x61000027;
    pub const DT_SCE_UNK24: i64 = 0x61000028;
    pub const DT_SCE_JMPREL: i64 = 0x61000029;
    pub const DT_SCE_UNK25: i64 = 0x6100002a;
    pub const DT_SCE_PLTREL: i64 = 0x6100002b;
    pub const DT_SCE_UNK26: i64 = 0x6100002c;
    pub const DT_SCE_PLTRELSZ: i64 = 0x6100002d;
    pub const DT_SCE_UNK27: i64 = 0x6100002e;
    pub const DT_SCE_RELA: i64 = 0x6100002f;
    pub const DT_SCE_UNK28: i64 = 0x61000030;
    pub const DT_SCE_RELASZ: i64 = 0x61000031;
    pub const DT_SCE_UNK29: i64 = 0x61000032;
    pub const DT_SCE_RELAENT: i64 = 0x61000033;
    pub const DT_SCE_UNK30: i64 = 0x61000034;
    pub const DT_SCE_STRTAB: i64 = 0x61000035;
    pub const DT_SCE_UNK31: i64 = 0x61000036;
    pub const DT_SCE_STRSZ: i64 = 0x61000037;
    pub const DT_SCE_UNK32: i64 = 0x61000038;
    pub const DT_SCE_SYMTAB: i64 = 0x61000039;
    pub const DT_SCE_UNK33: i64 = 0x6100003a;
    pub const DT_SCE_SYMENT: i64 = 0x6100003b;
    pub const DT_SCE_UNK34: i64 = 0x6100003c;
    pub const DT_SCE_HASHSZ: i64 = 0x6100003d;
    pub const DT_SCE_UNK35: i64 = 0x6100003e;
    pub const DT_SCE_SYMTABSZ: i64 = 0x6100003f;
    pub const DT_SCE_UNK36: i64 = 0x6ffffff9;
    pub const DT_SCE_UNK37: i64 = 0x6ffffffb;

    pub(super) fn parse(
        data: Vec<u8>,
        comment: Vec<u8>,
        dynoff: usize,
    ) -> Result<Self, FileInfoError> {
        // Parse dynamic.
        for entry in data[dynoff..].chunks(16) {
            let tag = LE::read_i64(entry);
            let value = &entry[8..];

            match tag {
                Self::DT_NULL => break,
                Self::DT_NEEDED => {}
                Self::DT_PLTRELSZ => {}
                Self::DT_PLTGOT => {}
                Self::DT_HASH => {}
                Self::DT_STRTAB => {}
                Self::DT_SYMTAB => {}
                Self::DT_RELA => {}
                Self::DT_RELASZ => {}
                Self::DT_RELAENT => {}
                Self::DT_STRSZ => {}
                Self::DT_SYMENT => {}
                Self::DT_INIT => {}
                Self::DT_FINI => {}
                Self::DT_SONAME => {}
                Self::DT_RPATH => {}
                Self::DT_SYMBOLIC => {}
                Self::DT_REL => {}
                Self::DT_RELSZ => {}
                Self::DT_RELENT => {}
                Self::DT_PLTREL => {}
                Self::DT_DEBUG => {}
                Self::DT_TEXTREL => {}
                Self::DT_JMPREL => {}
                Self::DT_BIND_NOW => {}
                Self::DT_INIT_ARRAY => {}
                Self::DT_FINI_ARRAY => {}
                Self::DT_INIT_ARRAYSZ => {}
                Self::DT_FINI_ARRAYSZ => {}
                Self::DT_RUNPATH => {}
                Self::DT_FLAGS => {}
                Self::DT_ENCODING => {}
                Self::DT_PREINIT_ARRAY => {}
                Self::DT_PREINIT_ARRAYSZ => {}
                Self::DT_SCE_UNK1 => {}
                Self::DT_SCE_FINGERPRINT => {}
                Self::DT_SCE_UNK2 => {}
                Self::DT_SCE_UNK3 => {}
                Self::DT_SCE_UNK4 => {}
                Self::DT_SCE_UNK5 => {}
                Self::DT_SCE_UNK6 => {}
                Self::DT_SCE_FILENAME => {}
                Self::DT_SCE_MODULE_INFO => {}
                Self::DT_SCE_NEEDED_MODULE => {}
                Self::DT_SCE_UNK7 => {}
                Self::DT_SCE_MODULE_ATTR => {}
                Self::DT_SCE_UNK8 => {}
                Self::DT_SCE_EXPORT_LIB => {}
                Self::DT_SCE_UNK9 => {}
                Self::DT_SCE_IMPORT_LIB => {}
                Self::DT_SCE_UNK10 => {}
                Self::DT_SCE_EXPORT_LIB_ATTR => {}
                Self::DT_SCE_UNK11 => {}
                Self::DT_SCE_IMPORT_LIB_ATTR => {}
                Self::DT_SCE_UNK12 => {}
                Self::DT_SCE_UNK13 => {}
                Self::DT_SCE_UNK14 => {}
                Self::DT_SCE_UNK15 => {}
                Self::DT_SCE_UNK16 => {}
                Self::DT_SCE_UNK17 => {}
                Self::DT_SCE_UNK18 => {}
                Self::DT_SCE_UNK19 => {}
                Self::DT_SCE_UNK20 => {}
                Self::DT_SCE_UNK21 => {}
                Self::DT_SCE_UNK22 => {}
                Self::DT_SCE_HASH => {}
                Self::DT_SCE_UNK23 => {}
                Self::DT_SCE_PLTGOT => {}
                Self::DT_SCE_UNK24 => {}
                Self::DT_SCE_JMPREL => {}
                Self::DT_SCE_UNK25 => {}
                Self::DT_SCE_PLTREL => {}
                Self::DT_SCE_UNK26 => {}
                Self::DT_SCE_PLTRELSZ => {}
                Self::DT_SCE_UNK27 => {}
                Self::DT_SCE_RELA => {}
                Self::DT_SCE_UNK28 => {}
                Self::DT_SCE_RELASZ => {}
                Self::DT_SCE_UNK29 => {}
                Self::DT_SCE_RELAENT => {}
                Self::DT_SCE_UNK30 => {}
                Self::DT_SCE_STRTAB => {}
                Self::DT_SCE_UNK31 => {}
                Self::DT_SCE_STRSZ => {}
                Self::DT_SCE_UNK32 => {}
                Self::DT_SCE_SYMTAB => {}
                Self::DT_SCE_UNK33 => {}
                Self::DT_SCE_SYMENT => {}
                Self::DT_SCE_UNK34 => {}
                Self::DT_SCE_HASHSZ => {}
                Self::DT_SCE_UNK35 => {}
                Self::DT_SCE_SYMTABSZ => {}
                Self::DT_SCE_UNK36 => {}
                Self::DT_SCE_UNK37 => {}
                v => return Err(FileInfoError::UnknownTag(v)),
            }
        }

        Ok(Self { data, comment })
    }
}

/// Represents an error for file info parsing.
#[derive(Debug, Error)]
pub enum FileInfoError {
    #[error("unknown tag {0:#018x}")]
    UnknownTag(i64),
}
