use self::inode::Inode;
use crate::header::PfsFlags;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::uninit;

mod block;
pub mod header;
mod inode;

pub struct Reader<'image> {
    header: header::Header,
    inodes: Vec<Inode>,
    block_reader: Box<dyn block::Reader<'image> + 'image>,
    block_buffer: Vec<u8>, // A temporary buffer with the size of header.blocksize() to read data.
}

impl<'image> Reader<'image> {
    pub fn new(image: &'image [u8], image_flags: PfsFlags, ekpfs: &[u8]) -> Result<Self, NewError> {
        // Read header.
        let header = match header::Header::read(image) {
            Ok(v) => v,
            Err(e) => return Err(NewError::InvalidHeader(e)),
        };

        // Construct block reader.
        let block_size = header.block_size();
        let block_reader: Box<dyn block::Reader<'image> + 'image> = if header.mode().is_encrypted()
        {
            let key_seed = header.key_seed();
            let (data_key, tweak_key) = Self::derive_block_key(image_flags, ekpfs, key_seed);

            Box::new(block::EncryptedReader::new(
                image, data_key, tweak_key, block_size,
            ))
        } else {
            Box::new(block::UnencryptedReader::new(image))
        };

        // Read inode blocks.
        let mut reader = Self {
            inodes: Vec::with_capacity(header.inode_count()),
            header,
            block_reader,
            block_buffer: util::mem::new_buffer::<u8>(block_size),
        };

        for block_num in 0..reader.header.inode_block_count() {
            let completed = match reader.read_inode_block(block_num) {
                Ok(v) => v,
                Err(e) => return Err(NewError::ReadBlockFailed(block_num + 1, e)),
            };

            if completed {
                break;
            }
        }

        Ok(reader)
    }

    /// Gets data key and tweak key.
    fn derive_block_key(
        image_flags: PfsFlags,
        ekpfs: &[u8],
        seed: &[u8; 16],
    ) -> ([u8; 16], [u8; 16]) {
        // Derive EKPFS from seed if PFS use new encryption.
        let ekpfs: Vec<u8> = if image_flags.is_new_encryption() {
            let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs).unwrap();

            hmac.update(seed);

            hmac.finalize().into_bytes().to_vec()
        } else {
            ekpfs.into()
        };

        // Derive key.
        let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs.as_slice()).unwrap();
        let mut input: Vec<u8> = Vec::with_capacity(seed.len() + 4);

        input.extend(1u32.to_le_bytes());
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

    fn read_inode_block(&mut self, block_num: usize) -> Result<bool, block::ReadError> {
        // Read block.
        let block_size = self.header.block_size();
        let offset = block_size + block_num * block_size;

        if let Err(e) = self
            .block_reader
            .read(offset, self.block_buffer.as_mut_slice())
        {
            return Err(e);
        }

        let mut block_data = self.block_buffer.as_slice();

        // Read inodes in the block.
        let reader = if self.header.mode().is_signed() {
            Inode::read_signed
        } else {
            Inode::read_unsigned
        };

        while self.inodes.len() < self.header.inode_count() {
            let inode = match reader(&mut block_data) {
                Ok(v) => v,
                Err(e) => match e {
                    inode::ReadError::TooSmall => return Ok(false),
                    inode::ReadError::IoFailed(e) => {
                        panic!("Failed to read inode from a buffer: {}", e)
                    }
                },
            };

            self.inodes.push(inode);
        }

        Ok(true)
    }
}

#[derive(Debug)]
pub enum NewError {
    InvalidHeader(header::ReadError),
    ReadBlockFailed(usize, block::ReadError),
}

impl Error for NewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidHeader(e) => Some(e),
            Self::ReadBlockFailed(_, e) => Some(e),
        }
    }
}

impl Display for NewError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(_) => f.write_str("invalid header"),
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
        }
    }
}
