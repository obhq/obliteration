use crate::header::Header;
use crate::Image;
use aes::Aes128;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::cmp::min;
use std::io::{IoSliceMut, Read, Seek, SeekFrom};
use util::mem::uninit;
use xts_mode::{get_tweak_default, Xts128};

pub(super) const XTS_BLOCK_SIZE: usize = 0x1000;

/// Gets data key and tweak key.
pub(super) fn get_xts_keys(ekpfs: &[u8], seed: &[u8; 16]) -> ([u8; 16], [u8; 16]) {
    // Derive key.
    let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs).unwrap();
    let mut input: Vec<u8> = Vec::with_capacity(seed.len() + 4);

    input.extend([0x01, 0x00, 0x00, 0x00]);
    input.extend(seed);

    hmac.update(input.as_slice());

    // Split key.
    let secret = hmac.finalize().into_bytes();
    let mut data_key: [u8; 16] = unsafe { uninit() };
    let mut tweak_key: [u8; 16] = unsafe { uninit() };

    tweak_key.copy_from_slice(&secret[..16]);
    data_key.copy_from_slice(&secret[16..]);

    (data_key, tweak_key)
}

/// Encapsulate an unencrypted PFS image.
pub(super) struct Unencrypted<I: Read + Seek> {
    image: I,
    header: Header,
}

impl<I: Read + Seek> Unencrypted<I> {
    pub fn new(image: I, header: Header) -> Self {
        Self { image, header }
    }
}

impl<I: Read + Seek> Image for Unencrypted<I> {
    fn header(&self) -> &Header {
        &self.header
    }
}

impl<I: Read + Seek> Seek for Unencrypted<I> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.image.seek(pos)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.image.rewind()
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.image.stream_position()
    }
}

impl<I: Read + Seek> Read for Unencrypted<I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.image.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.image.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.image.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.image.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.image.read_exact(buf)
    }
}

/// Encapsulate an encrypted PFS image.
pub(super) struct Encrypted<I: Read + Seek> {
    image: I,
    len: u64,
    header: Header,
    decryptor: Xts128<Aes128>,
    encrypted_start: usize, // Block index, not offset.
    offset: u64,
    current_block: Vec<u8>,
}

impl<I: Read + Seek> Encrypted<I> {
    pub fn new(
        image: I,
        image_len: u64,
        header: Header,
        decryptor: Xts128<Aes128>,
        encrypted_start: usize,
        current_offset: u64,
    ) -> Self {
        Self {
            image,
            len: image_len,
            header,
            decryptor,
            encrypted_start,
            offset: current_offset,
            current_block: Vec::new(),
        }
    }
}

impl<I: Read + Seek> Image for Encrypted<I> {
    fn header(&self) -> &Header {
        &self.header
    }
}

impl<I: Read + Seek> Seek for Encrypted<I> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        use std::io::{Error, ErrorKind};

        // Calculate the offset.
        let offset = match pos {
            SeekFrom::Start(v) => min(v, self.len),
            SeekFrom::End(v) => {
                if v >= 0 {
                    self.len
                } else {
                    match self.len.checked_sub(v.unsigned_abs()) {
                        Some(v) => v,
                        None => return Err(Error::from(ErrorKind::InvalidInput)),
                    }
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    min(self.offset + (v as u64), self.len)
                } else {
                    match self.offset.checked_sub(v.unsigned_abs()) {
                        Some(v) => v,
                        None => return Err(Error::from(ErrorKind::InvalidInput)),
                    }
                }
            }
        };

        // Update the offset if it is difference.
        if offset != self.offset {
            self.offset = offset;
            self.current_block.clear();
        }

        Ok(offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        if self.offset != 0 {
            self.offset = 0;
            self.current_block.clear();
        }

        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.offset)
    }
}

impl<I: Read + Seek> Read for Encrypted<I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::io::{Error, ErrorKind};

        // Check if we need to do the actual read.
        if buf.is_empty() || self.offset == self.len {
            return Ok(0);
        }

        // Fill output buffer until it is full or EOF.
        let mut copied = 0;

        loop {
            // Check if we have remaining data available for current block.
            if self.current_block.is_empty() {
                // Get offset for current block.
                let block = (self.offset as usize) / XTS_BLOCK_SIZE;
                let offset = (block * XTS_BLOCK_SIZE) as u64;

                // Seek image file to the target offset.
                match self.image.seek(SeekFrom::Start(offset)) {
                    Ok(v) => {
                        if v != offset {
                            return Err(Error::new(
                                ErrorKind::Other,
                                format!("unable to seek to offset {}", offset),
                            ));
                        }
                    }
                    Err(e) => return Err(e),
                }

                // Read the current block.
                self.current_block.reserve(XTS_BLOCK_SIZE);

                unsafe {
                    let buf = std::slice::from_raw_parts_mut(
                        self.current_block.as_mut_ptr(),
                        XTS_BLOCK_SIZE,
                    );

                    self.image.read_exact(buf)?;
                    self.current_block.set_len(XTS_BLOCK_SIZE);
                }

                // Decrypt block.
                if block >= self.encrypted_start {
                    let tweak = get_tweak_default(block as _);

                    self.decryptor
                        .decrypt_sector(&mut self.current_block, tweak);
                }

                // Discard any data before current offset.
                let offset = (self.offset as usize) % XTS_BLOCK_SIZE;

                self.current_block.drain(..offset);
            }

            // Copy data to output buffer.
            let amount = min(buf.len() - copied, self.current_block.len());
            let src = self.current_block.drain(..amount);
            let dst = &mut buf[copied..(copied + amount)];

            dst.copy_from_slice(src.as_slice());
            copied += amount;

            // Advance the offset.
            drop(src);
            self.offset += amount as u64;

            // Check if output buffer is filled or EOF.
            if copied == buf.len() || self.offset == self.len {
                break Ok(copied);
            }
        }
    }
}
