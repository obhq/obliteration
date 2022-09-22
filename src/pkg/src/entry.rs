use crate::Pkg;
use util::mem::{read_u32_be, write_u32_be};

pub struct Entry<'c, 'p> {
    pkg: &'p Pkg<'c>,
    id: u32,
    filename_offset: u32,
    flags1: u32,
    flags2: u32,
    offset: u32,
    size: u32,
}

impl<'c, 'p> Entry<'c, 'p> {
    pub const RAW_SIZE: usize = 32;

    pub const KEYS: u32 = 0x00000010;
    pub const PARAM_SFO: u32 = 0x00001000;
    pub const PIC1_PNG: u32 = 0x00001006;
    pub const ICON0_PNG: u32 = 0x00001200;

    pub fn read(pkg: &'p Pkg<'c>, table: *const u8, index: usize) -> Self {
        let raw = unsafe { table.offset((index * Self::RAW_SIZE) as _) };
        let id = read_u32_be(raw, 0);
        let filename_offset = read_u32_be(raw, 4);
        let flags1 = read_u32_be(raw, 8);
        let flags2 = read_u32_be(raw, 12);
        let offset = read_u32_be(raw, 16);
        let size = read_u32_be(raw, 20);

        Self {
            pkg,
            id,
            filename_offset,
            flags1,
            flags2,
            offset,
            size,
        }
    }

    pub fn pkg(&self) -> &'p Pkg<'c> {
        self.pkg
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

pub struct EntryKey {
    seed: [u8; 32],
    digests: [[u8; 32]; 7],
    keys: [[u8; 256]; 7],
}

impl EntryKey {
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
