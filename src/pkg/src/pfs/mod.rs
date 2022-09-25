use crate::header::PfsFlags;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::uninit;

mod block;
pub mod header;

pub struct Reader<'image> {
    block_reader: Box<dyn block::Reader<'image> + 'image>,
}

impl<'image> Reader<'image> {
    pub fn new(image: &'image [u8], image_flags: PfsFlags, ekpfs: &[u8]) -> Result<Self, NewError> {
        // Read header.
        let header = match header::Header::read(image) {
            Ok(v) => v,
            Err(e) => return Err(NewError::InvalidHeader(e)),
        };

        let block_size = header.block_size();

        // Construct block reader.
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

        Ok(Self { block_reader })
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
}

#[derive(Debug)]
pub enum NewError {
    InvalidHeader(header::ReadError),
}

impl Error for NewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidHeader(e) => Some(e),
        }
    }
}

impl Display for NewError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(_) => f.write_str("invalid header"),
        }
    }
}
