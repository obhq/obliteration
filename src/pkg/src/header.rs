use util::mem::{read_u32_be, read_u64_be};

pub struct Header {
    entry_count: u32,
    table_offset: u32,
    pfs_flags: PfsFlags,
    pfs_offset: u64,
    pfs_size: u64,
}

impl Header {
    pub fn read(pkg: &[u8]) -> Result<Self, ReadError> {
        // Check size first so we can read without checking bound.
        if pkg.len() < 0x1000 {
            return Err(ReadError::TooSmall);
        }

        let pkg = pkg.as_ptr();

        // Check magic.
        let magic = read_u32_be(pkg, 0);

        if magic != 0x7f434e54 {
            return Err(ReadError::InvalidMagic);
        }

        // Read fields.
        let entry_count = read_u32_be(pkg, 0x10);
        let table_offset = read_u32_be(pkg, 0x18);
        let pfs_flags = PfsFlags(read_u64_be(pkg, 0x408));
        let pfs_offset = read_u64_be(pkg, 0x410);
        let pfs_size = read_u64_be(pkg, 0x418);

        Ok(Self {
            entry_count,
            table_offset,
            pfs_flags,
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

    pub fn pfs_flags(&self) -> PfsFlags {
        self.pfs_flags
    }

    pub fn pfs_offset(&self) -> usize {
        self.pfs_offset as _
    }

    pub fn pfs_size(&self) -> usize {
        self.pfs_size as _
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PfsFlags(u64);

impl PfsFlags {
    pub fn is_new_encryption(&self) -> bool {
        self.0 & 0x2000000000000000 != 0
    }
}

pub enum ReadError {
    TooSmall,
    InvalidMagic,
}
