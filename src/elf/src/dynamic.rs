use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use std::fmt::{Display, Formatter};

/// An iterator over the `PT_DYNAMIC`.
pub struct DynamicEntries<'a> {
    next: &'a [u8],
}

impl<'a> DynamicEntries<'a> {
    pub(crate) fn new(next: &'a [u8]) -> Self {
        Self { next }
    }
}

impl<'a> Iterator for DynamicEntries<'a> {
    type Item = (DynamicTag, [u8; 8]);

    fn next(&mut self) -> Option<Self::Item> {
        // Check if all entries has been read.
        if self.next.is_empty() {
            return None;
        }

        // Read the entry.
        let tag = LE::read_i64(self.next);
        let value = self.next[8..16].try_into().unwrap();

        // Move to next entry.
        self.next = &self.next[16..];

        Some((DynamicTag(tag), value))
    }
}

/// Tag of each dynamic entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicTag(i64);

impl DynamicTag {
    pub const DT_NULL: Self = Self(0);
    pub const DT_NEEDED: Self = Self(1);
    pub const DT_PLTRELSZ: Self = Self(2);
    pub const DT_PLTGOT: Self = Self(3);
    pub const DT_HASH: Self = Self(4);
    pub const DT_STRTAB: Self = Self(5);
    pub const DT_SYMTAB: Self = Self(6);
    pub const DT_RELA: Self = Self(7);
    pub const DT_RELASZ: Self = Self(8);
    pub const DT_RELAENT: Self = Self(9);
    pub const DT_STRSZ: Self = Self(10);
    pub const DT_SYMENT: Self = Self(11);
    pub const DT_INIT: Self = Self(12);
    pub const DT_FINI: Self = Self(13);
    pub const DT_SONAME: Self = Self(14);
    pub const DT_RPATH: Self = Self(15);
    pub const DT_SYMBOLIC: Self = Self(16);
    pub const DT_REL: Self = Self(17);
    pub const DT_RELSZ: Self = Self(18);
    pub const DT_RELENT: Self = Self(19);
    pub const DT_PLTREL: Self = Self(20);
    pub const DT_DEBUG: Self = Self(21);
    pub const DT_TEXTREL: Self = Self(22);
    pub const DT_JMPREL: Self = Self(23);
    pub const DT_BIND_NOW: Self = Self(24);
    pub const DT_INIT_ARRAY: Self = Self(25);
    pub const DT_FINI_ARRAY: Self = Self(26);
    pub const DT_INIT_ARRAYSZ: Self = Self(27);
    pub const DT_FINI_ARRAYSZ: Self = Self(28);
    pub const DT_RUNPATH: Self = Self(29);
    pub const DT_FLAGS: Self = Self(30);
    pub const DT_ENCODING: Self = Self(31);
    pub const DT_PREINIT_ARRAY: Self = Self(32);
    pub const DT_PREINIT_ARRAYSZ: Self = Self(33);
    pub const DT_SCE_UNK1: Self = Self(0x60000005);
    pub const DT_SCE_FINGERPRINT: Self = Self(0x61000007);
    pub const DT_SCE_UNK2: Self = Self(0x61000008);
    pub const DT_SCE_UNK3: Self = Self(0x6100000a);
    pub const DT_SCE_UNK4: Self = Self(0x6100000b);
    pub const DT_SCE_UNK5: Self = Self(0x6100000c);
    pub const DT_SCE_UNK6: Self = Self(0x6100000e);
    pub const DT_SCE_FILENAME: Self = Self(0x61000009);
    pub const DT_SCE_MODULE_INFO: Self = Self(0x6100000d);
    pub const DT_SCE_NEEDED_MODULE: Self = Self(0x6100000f);
    pub const DT_SCE_UNK7: Self = Self(0x61000010);
    pub const DT_SCE_MODULE_ATTR: Self = Self(0x61000011);
    pub const DT_SCE_UNK8: Self = Self(0x61000012);
    pub const DT_SCE_EXPORT_LIB: Self = Self(0x61000013);
    pub const DT_SCE_UNK9: Self = Self(0x61000014);
    pub const DT_SCE_IMPORT_LIB: Self = Self(0x61000015);
    pub const DT_SCE_UNK10: Self = Self(0x61000016);
    pub const DT_SCE_EXPORT_LIB_ATTR: Self = Self(0x61000017);
    pub const DT_SCE_UNK11: Self = Self(0x61000018);
    pub const DT_SCE_IMPORT_LIB_ATTR: Self = Self(0x61000019);
    pub const DT_SCE_UNK12: Self = Self(0x6100001a);
    pub const DT_SCE_UNK13: Self = Self(0x6100001b);
    pub const DT_SCE_UNK14: Self = Self(0x6100001c);
    pub const DT_SCE_UNK15: Self = Self(0x6100001d);
    pub const DT_SCE_UNK16: Self = Self(0x6100001e);
    pub const DT_SCE_UNK17: Self = Self(0x6100001f);
    pub const DT_SCE_UNK18: Self = Self(0x61000020);
    pub const DT_SCE_UNK19: Self = Self(0x61000021);
    pub const DT_SCE_UNK20: Self = Self(0x61000022);
    pub const DT_SCE_UNK21: Self = Self(0x61000023);
    pub const DT_SCE_UNK22: Self = Self(0x61000024);
    pub const DT_SCE_HASH: Self = Self(0x61000025);
    pub const DT_SCE_UNK23: Self = Self(0x61000026);
    pub const DT_SCE_PLTGOT: Self = Self(0x61000027);
    pub const DT_SCE_UNK24: Self = Self(0x61000028);
    pub const DT_SCE_JMPREL: Self = Self(0x61000029);
    pub const DT_SCE_UNK25: Self = Self(0x6100002a);
    pub const DT_SCE_PLTREL: Self = Self(0x6100002b);
    pub const DT_SCE_UNK26: Self = Self(0x6100002c);
    pub const DT_SCE_PLTRELSZ: Self = Self(0x6100002d);
    pub const DT_SCE_UNK27: Self = Self(0x6100002e);
    pub const DT_SCE_RELA: Self = Self(0x6100002f);
    pub const DT_SCE_UNK28: Self = Self(0x61000030);
    pub const DT_SCE_RELASZ: Self = Self(0x61000031);
    pub const DT_SCE_UNK29: Self = Self(0x61000032);
    pub const DT_SCE_RELAENT: Self = Self(0x61000033);
    pub const DT_SCE_UNK30: Self = Self(0x61000034);
    pub const DT_SCE_STRTAB: Self = Self(0x61000035);
    pub const DT_SCE_UNK31: Self = Self(0x61000036);
    pub const DT_SCE_STRSZ: Self = Self(0x61000037);
    pub const DT_SCE_UNK32: Self = Self(0x61000038);
    pub const DT_SCE_SYMTAB: Self = Self(0x61000039);
    pub const DT_SCE_UNK33: Self = Self(0x6100003a);
    pub const DT_SCE_SYMENT: Self = Self(0x6100003b);
    pub const DT_SCE_UNK34: Self = Self(0x6100003c);
    pub const DT_SCE_HASHSZ: Self = Self(0x6100003d);
    pub const DT_SCE_UNK35: Self = Self(0x6100003e);
    pub const DT_SCE_SYMTABSZ: Self = Self(0x6100003f);
    pub const DT_SCE_UNK36: Self = Self(0x6ffffff9);
    pub const DT_SCE_UNK37: Self = Self(0x6ffffffb);
}

impl Display for DynamicTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::DT_NULL => f.write_str("DT_NULL"),
            Self::DT_NEEDED => f.write_str("DT_NEEDED"),
            Self::DT_PLTRELSZ => f.write_str("DT_PLTRELSZ"),
            Self::DT_PLTGOT => f.write_str("DT_PLTGOT"),
            Self::DT_HASH => f.write_str("DT_HASH"),
            Self::DT_STRTAB => f.write_str("DT_STRTAB"),
            Self::DT_SYMTAB => f.write_str("DT_SYMTAB"),
            Self::DT_RELA => f.write_str("DT_RELA"),
            Self::DT_RELASZ => f.write_str("DT_RELASZ"),
            Self::DT_RELAENT => f.write_str("DT_RELAENT"),
            Self::DT_STRSZ => f.write_str("DT_STRSZ"),
            Self::DT_SYMENT => f.write_str("DT_SYMENT"),
            Self::DT_INIT => f.write_str("DT_INIT"),
            Self::DT_FINI => f.write_str("DT_FINI"),
            Self::DT_SONAME => f.write_str("DT_SONAME"),
            Self::DT_RPATH => f.write_str("DT_RPATH"),
            Self::DT_SYMBOLIC => f.write_str("DT_SYMBOLIC"),
            Self::DT_REL => f.write_str("DT_REL"),
            Self::DT_RELSZ => f.write_str("DT_RELSZ"),
            Self::DT_RELENT => f.write_str("DT_RELENT"),
            Self::DT_PLTREL => f.write_str("DT_PLTREL"),
            Self::DT_DEBUG => f.write_str("DT_DEBUG"),
            Self::DT_TEXTREL => f.write_str("DT_TEXTREL"),
            Self::DT_JMPREL => f.write_str("DT_JMPREL"),
            Self::DT_BIND_NOW => f.write_str("DT_BIND_NOW"),
            Self::DT_INIT_ARRAY => f.write_str("DT_INIT_ARRAY"),
            Self::DT_FINI_ARRAY => f.write_str("DT_FINI_ARRAY"),
            Self::DT_INIT_ARRAYSZ => f.write_str("DT_INIT_ARRAYSZ"),
            Self::DT_FINI_ARRAYSZ => f.write_str("DT_FINI_ARRAYSZ"),
            Self::DT_RUNPATH => f.write_str("DT_RUNPATH"),
            Self::DT_FLAGS => f.write_str("DT_FLAGS"),
            Self::DT_ENCODING => f.write_str("DT_ENCODING"),
            Self::DT_PREINIT_ARRAY => f.write_str("DT_PREINIT_ARRAY"),
            Self::DT_PREINIT_ARRAYSZ => f.write_str("DT_PREINIT_ARRAYSZ"),
            Self::DT_SCE_FINGERPRINT => f.write_str("DT_SCE_FINGERPRINT"),
            Self::DT_SCE_FILENAME => f.write_str("DT_SCE_FILENAME"),
            Self::DT_SCE_MODULE_INFO => f.write_str("DT_SCE_MODULE_INFO"),
            Self::DT_SCE_NEEDED_MODULE => f.write_str("DT_SCE_NEEDED_MODULE"),
            Self::DT_SCE_MODULE_ATTR => f.write_str("DT_SCE_MODULE_ATTR"),
            Self::DT_SCE_EXPORT_LIB => f.write_str("DT_SCE_EXPORT_LIB"),
            Self::DT_SCE_IMPORT_LIB => f.write_str("DT_SCE_IMPORT_LIB"),
            Self::DT_SCE_EXPORT_LIB_ATTR => f.write_str("DT_SCE_EXPORT_LIB_ATTR"),
            Self::DT_SCE_IMPORT_LIB_ATTR => f.write_str("DT_SCE_IMPORT_LIB_ATTR"),
            Self::DT_SCE_HASH => f.write_str("DT_SCE_HASH"),
            Self::DT_SCE_PLTGOT => f.write_str("DT_SCE_PLTGOT"),
            Self::DT_SCE_JMPREL => f.write_str("DT_SCE_JMPREL"),
            Self::DT_SCE_PLTREL => f.write_str("DT_SCE_PLTREL"),
            Self::DT_SCE_PLTRELSZ => f.write_str("DT_SCE_PLTRELSZ"),
            Self::DT_SCE_RELA => f.write_str("DT_SCE_RELA"),
            Self::DT_SCE_RELASZ => f.write_str("DT_SCE_RELASZ"),
            Self::DT_SCE_RELAENT => f.write_str("DT_SCE_RELAENT"),
            Self::DT_SCE_STRTAB => f.write_str("DT_SCE_STRTAB"),
            Self::DT_SCE_STRSZ => f.write_str("DT_SCE_STRSZ"),
            Self::DT_SCE_SYMTAB => f.write_str("DT_SCE_SYMTAB"),
            Self::DT_SCE_SYMENT => f.write_str("DT_SCE_SYMENT"),
            Self::DT_SCE_HASHSZ => f.write_str("DT_SCE_HASHSZ"),
            Self::DT_SCE_SYMTABSZ => f.write_str("DT_SCE_SYMTABSZ"),
            v => write!(f, "{:#018x}", v.0),
        }
    }
}

bitflags! {
    /// Contains flags for the module.
    #[derive(Debug, Clone, Copy)]
    pub struct DynamicFlags: u64 {
        const DF_SYMBOLIC = 0x02; // Not used in PS4.
        const DF_TEXTREL = 0x04;
        const DF_BIND_NOW = 0x08; // Not used in PS4.
    }
}

impl Display for DynamicFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
