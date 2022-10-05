use crate::header::Header;
use crate::{Image, ReadError};
use aes::Aes128;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use util::mem::uninit;
use xts_mode::{get_tweak_default, Xts128};

pub(super) const XTS_BLOCK_SIZE: usize = 0x1000;

/// Gets data key and tweak key.
pub(super) fn get_xts_keys(ekpfs: &[u8], seed: &[u8; 16]) -> ([u8; 16], [u8; 16]) {
    // Derive key.
    let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs).unwrap();
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

pub(super) struct Unencrypted<R: AsRef<[u8]>> {
    raw: R,
    header: Header,
}

impl<R: AsRef<[u8]>> Unencrypted<R> {
    pub fn new(raw: R, header: Header) -> Self {
        Self { raw, header }
    }
}

impl<R: AsRef<[u8]>> Image for Unencrypted<R> {
    fn header(&self) -> &Header {
        &self.header
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError> {
        let block = match self.raw.as_ref().get(offset..(offset + buf.len())) {
            Some(v) => v,
            None => return Err(ReadError::InvalidOffset),
        };

        buf.copy_from_slice(block);

        Ok(())
    }
}

pub(super) struct Encrypted<R: AsRef<[u8]>> {
    raw: R,
    header: Header,
    decryptor: Xts128<Aes128>,
    encrypted_start: usize,
}

impl<R: AsRef<[u8]>> Encrypted<R> {
    pub fn new(raw: R, header: Header, decryptor: Xts128<Aes128>, encrypted_start: usize) -> Self {
        Self {
            raw,
            header,
            decryptor,
            encrypted_start,
        }
    }
}

impl<R: AsRef<[u8]>> Encrypted<R> {
    /// Fill `buf` with decrypted data.
    fn read_xts_block(&self, num: usize, buf: &mut [u8; XTS_BLOCK_SIZE]) -> Result<(), ReadError> {
        // Read block.
        let offset = num * XTS_BLOCK_SIZE;
        let data = match self.raw.as_ref().get(offset..(offset + XTS_BLOCK_SIZE)) {
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

impl<R: AsRef<[u8]>> Image for Encrypted<R> {
    fn header(&self) -> &Header {
        &self.header
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError> {
        // Read a first block for destination offset.
        let mut block_num = offset / XTS_BLOCK_SIZE;
        let mut block_data: [u8; XTS_BLOCK_SIZE] = uninit();

        self.read_xts_block(block_num, &mut block_data)?;

        // Fill output buffer.
        let mut src = &block_data[(offset % XTS_BLOCK_SIZE)..];
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

            self.read_xts_block(block_num, &mut block_data)?;

            src = &block_data;
        }

        Ok(())
    }
}
