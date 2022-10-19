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
}
