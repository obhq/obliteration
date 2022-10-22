use util::mem::{read_u32_le, read_u64_le};

pub struct Entry {
    flags: u32,
    offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
}

impl Entry {
    pub(super) const RAW_SIZE: usize = 32;

    pub(super) fn read(data: *const u8) -> Self {
        let flags = read_u32_le(data, 0);
        let offset = read_u64_le(data, 8);
        let compressed_size = read_u64_le(data, 16);
        let uncompressed_size = read_u64_le(data, 24);

        Self {
            flags,
            offset,
            compressed_size,
            uncompressed_size,
        }
    }

    pub fn id(&self) -> u16 {
        (self.flags >> 20) as u16
    }

    pub fn is_compressed(&self) -> bool {
        (self.flags & 8) != 0
    }

    pub fn is_blocked(&self) -> bool {
        (self.flags & 0x800) != 0
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    pub fn uncompressed_size(&self) -> u64 {
        self.uncompressed_size
    }
}
