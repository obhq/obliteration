use aes::cipher::generic_array::GenericArray;
use aes::cipher::KeyInit;
use aes::Aes128;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::uninit;
use xts_mode::{get_tweak_default, Xts128};

pub trait Reader<'image> {
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

impl<'image> Reader<'image> for UnencryptedReader<'image> {
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
    data_key: [u8; 16],
    tweak_key: [u8; 16],
    encrypted_start: usize,
}

impl<'image> EncryptedReader<'image> {
    const SECTOR_SIZE: usize = 0x1000;

    pub fn new(
        image: &'image [u8],
        data_key: [u8; 16],
        tweak_key: [u8; 16],
        block_size: usize,
    ) -> Self {
        Self {
            image,
            data_key,
            tweak_key,
            encrypted_start: block_size / Self::SECTOR_SIZE,
        }
    }

    /// Read the specified sector. This method always read the whole sector; thus `buf` is always filled.
    fn read_sector(&self, num: usize, buf: &mut [u8; 0x1000]) -> Result<(), ReadError> {
        let offset = num * Self::SECTOR_SIZE;
        let sector = match self.image.get(offset..(offset + Self::SECTOR_SIZE)) {
            Some(v) => v,
            None => return Err(ReadError::InvalidOffset),
        };

        buf.copy_from_slice(sector);

        if num >= self.encrypted_start {
            self.decrypt_sector(num, buf);
        }

        Ok(())
    }

    fn decrypt_sector(&self, sector: usize, data: &mut [u8; 0x1000]) {
        let cipher_1 = Aes128::new(GenericArray::from_slice(&self.data_key));
        let cipher_2 = Aes128::new(GenericArray::from_slice(&self.tweak_key));
        let xts = Xts128::<Aes128>::new(cipher_1, cipher_2);
        let tweak = get_tweak_default(sector as _);

        xts.decrypt_sector(data, tweak);
    }
}

impl<'image> Reader<'image> for EncryptedReader<'image> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError> {
        let dest = buf.as_mut_ptr();
        let mut sector_num = offset / Self::SECTOR_SIZE;
        let mut sector_data: [u8; 0x1000] = uninit();
        let mut copied = 0;

        self.read_sector(sector_num, &mut sector_data)?;

        loop {
            let dest = unsafe { dest.offset(copied as _) };
            let need = buf.len() - copied;

            if need <= Self::SECTOR_SIZE {
                unsafe { dest.copy_from_nonoverlapping(sector_data.as_ptr(), need) };
                break;
            } else {
                unsafe { dest.copy_from_nonoverlapping(sector_data.as_ptr(), Self::SECTOR_SIZE) };
                copied += Self::SECTOR_SIZE;
            }

            sector_num += 1;

            self.read_sector(sector_num, &mut sector_data)?;
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
            Self::InvalidOffset => f.write_str("the specified offset is not valid"),
        }
    }
}
