use self::inode::Inode;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::{new_buffer, uninit};

pub mod header;
pub mod inode;
pub mod reader;

pub struct Pfs<'image> {
    reader: Box<dyn reader::Reader + 'image>,
    header: header::Header,
    inodes: Vec<Inode>,
}

impl<'image> Pfs<'image> {
    pub fn open(
        image: &'image [u8],
        flags: ImageFlags,
        ekpfs: Option<&[u8]>,
    ) -> Result<Self, OpenError> {
        // Read header.
        let header = match header::Header::read(image) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::InvalidHeader(e)),
        };

        // Construct reader.
        let block_size = header.block_size();
        let reader: Box<dyn reader::Reader + 'image> = if header.mode().is_encrypted() {
            let ekpfs = match ekpfs {
                Some(v) => v,
                None => panic!("The image is encrypted but no EKPFS is provided"),
            };

            let key_seed = header.key_seed();
            let (data_key, tweak_key) = Self::derive_block_key(flags, ekpfs, key_seed);
            let reader = reader::EncryptedReader::new(image, data_key, tweak_key, block_size);

            Box::new(reader)
        } else {
            Box::new(reader::UnencryptedReader::new(image))
        };

        // Read inode blocks.
        let mut pfs = Self {
            inodes: Vec::with_capacity(header.inode_count()),
            reader,
            header,
        };

        for block_num in 0..pfs.header.inode_block_count() {
            let completed = match pfs.load_inodes(block_num) {
                Ok(v) => v,
                Err(e) => return Err(OpenError::ReadBlockFailed(block_num + 1, e)),
            };

            if completed {
                break;
            }
        }

        Ok(pfs)
    }

    /// `block_num` is a number of inode block, not image block.
    fn load_inodes(&mut self, block_num: usize) -> Result<bool, reader::ReadError> {
        // Get the offset for target block. The first inode block always start at image second
        // block.
        let block_size = self.header.block_size();
        let offset = block_size + block_num * block_size;

        // Read the whole block.
        let mut block_data = new_buffer(block_size);

        if let Err(e) = self.reader.read(offset, block_data.as_mut_slice()) {
            return Err(e);
        }

        // Read inodes in the block.
        let mut src = block_data.as_slice();
        let reader = if self.header.mode().is_signed() {
            Inode::read_signed
        } else {
            Inode::read_unsigned
        };

        while self.inodes.len() < self.header.inode_count() {
            let inode = match reader(&mut src) {
                Ok(v) => v,
                Err(e) => match e {
                    inode::ReadError::TooSmall => return Ok(false),
                    inode::ReadError::IoFailed(e) => {
                        panic!("Failed to read inode from a buffer: {}", e);
                    }
                },
            };

            self.inodes.push(inode);
        }

        Ok(true)
    }

    /// Gets data key and tweak key.
    fn derive_block_key(flags: ImageFlags, ekpfs: &[u8], seed: &[u8; 16]) -> ([u8; 16], [u8; 16]) {
        // Derive EKPFS from seed if PFS use new encryption.
        let ekpfs: Vec<u8> = if flags.is_new_encryption() {
            let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs).unwrap();
            hmac.update(seed);
            hmac.finalize().into_bytes().to_vec()
        } else {
            ekpfs.into()
        };

        // Derive key.
        let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs.as_slice()).unwrap();
        let mut input: Vec<u8> = Vec::with_capacity(seed.len() + 4);

        input.extend(&[0x01, 0x00, 0x00, 0x00]);
        input.extend(seed);

        hmac.update(input.as_slice());

        // Split key.
        let secret = hmac.finalize().into_bytes();
        let mut data_key: [u8; 16] = uninit();
        let mut tweak_key: [u8; 16] = uninit();

        tweak_key.copy_from_slice(&secret[..16]);
        data_key.copy_from_slice(&secret[16..]);

        (data_key, tweak_key)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ImageFlags(u64);

impl ImageFlags {
    pub fn is_new_encryption(&self) -> bool {
        self.0 & 0x2000000000000000 != 0
    }
}

impl From<u64> for ImageFlags {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

#[derive(Debug)]
pub enum OpenError {
    InvalidHeader(header::ReadError),
    ReadBlockFailed(usize, reader::ReadError),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidHeader(e) => Some(e),
            Self::ReadBlockFailed(_, e) => Some(e),
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(_) => f.write_str("invalid header"),
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
        }
    }
}
