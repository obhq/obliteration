use crate::util::binary::read_u32_be;

pub struct Header {}

impl Header {
    pub fn read(pkg: &[u8]) -> Result<Self, ReadError> {
        // Check size first so we can read without checking bound.
        if pkg.len() < 0x1000 {
            return Err(ReadError::TooSmall);
        }

        let pkg = pkg.as_ptr();

        // Check magic.
        let magic = read_u32_be(pkg);

        if magic != 0x7f434e54 {
            return Err(ReadError::InvalidMagic);
        }

        Ok(Self {})
    }
}

pub enum ReadError {
    TooSmall,
    InvalidMagic,
}
