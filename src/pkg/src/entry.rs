use util::mem::{read_u32_be, write_u32_be};

pub struct Entry {
    id: u32,
    filename_offset: u32,
    flags1: u32,
    flags2: u32,
    data_offset: u32,
    data_size: u32,
}

impl Entry {
    pub const RAW_SIZE: usize = 32;

    pub const ENTRY_KEYS: u32 = 0x00000010;
    pub const PFS_IMAGE_KEY: u32 = 0x00000020;
    pub const PARAM_SFO: u32 = 0x00001000;
    pub const PIC1_PNG: u32 = 0x00001006;
    pub const ICON0_PNG: u32 = 0x00001200;

    pub fn read(raw: *const u8) -> Self {
        let id = read_u32_be(raw, 0);
        let filename_offset = read_u32_be(raw, 4);
        let flags1 = read_u32_be(raw, 8);
        let flags2 = read_u32_be(raw, 12);
        let data_offset = read_u32_be(raw, 16);
        let data_size = read_u32_be(raw, 20);

        Self {
            id,
            filename_offset,
            flags1,
            flags2,
            data_offset,
            data_size,
        }
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

    pub fn data_offset(&self) -> usize {
        self.data_offset as _
    }

    pub fn data_size(&self) -> usize {
        self.data_size as _
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let p = buf.as_mut_ptr();

        write_u32_be(p, 0, self.id);
        write_u32_be(p, 4, self.filename_offset);
        write_u32_be(p, 8, self.flags1);
        write_u32_be(p, 12, self.flags2);
        write_u32_be(p, 16, self.data_offset);
        write_u32_be(p, 20, self.data_size);

        buf
    }
}
