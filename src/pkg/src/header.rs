use byteorder::{ByteOrder, BE};

pub struct Header {
    entry_count: u32,
    table_offset: u32,
    pfs_offset: u64,
    pfs_size: u64,
}

impl Header {
    pub fn read(pkg: &[u8]) -> Result<Self, ReadError> {
        // Check size first so we can read without checking bound.
        if pkg.len() < 0x1000 {
            return Err(ReadError::TooSmall);
        }

        // Check magic.
        let magic = BE::read_u32(&pkg[0x00..]);

        if magic != 0x7f434e54 {
            return Err(ReadError::InvalidMagic);
        }

        // Read fields.
        let entry_count = BE::read_u32(&pkg[0x10..]);
        let table_offset = BE::read_u32(&pkg[0x18..]);
        let pfs_offset = BE::read_u64(&pkg[0x410..]);
        let pfs_size = BE::read_u64(&pkg[0x418..]);

        Ok(Self {
            entry_count,
            table_offset,
            pfs_offset,
            pfs_size,
        })
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count as _
    }

    pub fn table_offset(&self) -> usize {
        self.table_offset as _
    }

    pub fn pfs_offset(&self) -> usize {
        self.pfs_offset as _
    }

    pub fn pfs_size(&self) -> usize {
        self.pfs_size as _
    }
}

pub enum ReadError {
    TooSmall,
    InvalidMagic,
}
