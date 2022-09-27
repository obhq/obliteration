use aes::cipher::KeyInit;
use aes::Aes128;
use generic_array::GenericArray;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::uninit;
use xts_mode::{get_tweak_default, Xts128};

pub trait Reader {
    /// Fill `buf` from data at `offset`.
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError>;
}

pub struct UnencryptedReader<'image> {
    image: &'image [u8],
}

impl<'image> UnencryptedReader<'image> {
    pub fn new(image: &'image [u8]) -> Self {
        Self { image }
    }
}

impl<'image> Reader for UnencryptedReader<'image> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError> {
        let block = match self.image.get(offset..(offset + buf.len())) {
            Some(v) => v,
            None => return Err(ReadError::InvalidOffset),
        };

        buf.copy_from_slice(block);

        Ok(())
    }
}

pub struct EncryptedReader<'image> {
    image: &'image [u8],
    decryptor: Xts128<Aes128>,
    encrypted_start: usize,
}

impl<'image> EncryptedReader<'image> {
    const BLOCK_SIZE: usize = 0x1000;

    pub fn new(
        image: &'image [u8],
        data_key: [u8; 16],
        tweak_key: [u8; 16],
        block_size: usize,
    ) -> Self {
        // The super block is never encrypted.
        if block_size < Self::BLOCK_SIZE {
            panic!("Block size must not less than {}", Self::BLOCK_SIZE);
        }

        // Setup decryptor.
        let cipher_1 = Aes128::new(GenericArray::from_slice(&data_key));
        let cipher_2 = Aes128::new(GenericArray::from_slice(&tweak_key));

        Self {
            image,
            decryptor: Xts128::<Aes128>::new(cipher_1, cipher_2),
            encrypted_start: block_size / Self::BLOCK_SIZE,
        }
    }

    /// This method always read the whole block. So `buf` is always filled.
    fn read_block(&self, num: usize, buf: &mut [u8; 0x1000]) -> Result<(), ReadError> {
        // Read block.
        let offset = num * Self::BLOCK_SIZE;
        let data = match self.image.get(offset..(offset + Self::BLOCK_SIZE)) {
            Some(v) => v,
            None => return Err(ReadError::InvalidOffset),
        };

        buf.copy_from_slice(data);

        // Decrypt block.
        if num >= self.encrypted_start {
            let tweak = get_tweak_default(num as _);
            self.decryptor.decrypt_sector(buf, tweak);
        }

        Ok(())
    }
}

impl<'image> Reader for EncryptedReader<'image> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError> {
        // Read a first block for destination offset.
        let mut block_num = offset / Self::BLOCK_SIZE;
        let mut block_data: [u8; 0x1000] = uninit();

        self.read_block(block_num, &mut block_data)?;

        // Fill output buffer.
        let mut src = &block_data[(offset % Self::BLOCK_SIZE)..];
        let dest = buf.as_mut_ptr();
        let mut copied = 0;

        loop {
            let dest = unsafe { dest.offset(copied as _) };

            // Check if remaining block can fill the remaining buffer.
            let need = buf.len() - copied;

            if need <= src.len() {
                unsafe { dest.copy_from_nonoverlapping(src.as_ptr(), need) };
                break;
            } else {
                unsafe { dest.copy_from_nonoverlapping(src.as_ptr(), src.len()) };
                copied += src.len();
            }

            // Read next block.
            block_num += 1;

            self.read_block(block_num, &mut block_data)?;

            src = &block_data;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ReadError {
    InvalidOffset,
}

impl Error for ReadError {}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidOffset => f.write_str("invalid offset"),
        }
    }
}
