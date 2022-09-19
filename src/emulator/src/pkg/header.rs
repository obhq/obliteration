use crate::util::binary::{read_u32_be, read_u64_be};

pub struct Header {
    pfs_image_offset: u64,
    pfs_image_size: u64,
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
        let pfs_image_offset = read_u64_be(pkg, 0x410);
        let pfs_image_size = read_u64_be(pkg, 0x418);

        Ok(Self {
            pfs_image_offset,
            pfs_image_size,
        })
    }
}

pub enum ReadError {
    TooSmall,
    InvalidMagic,
}
