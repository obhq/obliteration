use crate::entry::Entry;
use flate2::read::ZlibDecoder;
use std::borrow::Cow;
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use util::mem::{read_u32_le, uninit};

pub(super) trait EntryReader: Read + Seek {}

pub(super) struct NonBlockedReader<'pup> {
    data: Cow<'pup, [u8]>,
    offset: u64,
}

impl<'pup> NonBlockedReader<'pup> {
    pub fn new(entry: &'pup Entry, pup: &'pup [u8]) -> Result<Self, ContiguousReaderError> {
        let offset = entry.offset() as usize;
        let size = entry.compressed_size() as usize;
        let data = match pup.get(offset..(offset + size)) {
            Some(v) if v.len() == size => v,
            _ => return Err(ContiguousReaderError::InvalidOffset),
        };

        Ok(Self {
            data: if entry.is_compressed() {
                // Non-blocked entry is for smaller data so we can keep the whole decompressed data
                // in the memory.
                let mut decoder = ZlibDecoder::new(data);
                let mut decompressed: Vec<u8> = Vec::new();

                if let Err(e) = decoder.read_to_end(&mut decompressed) {
                    return Err(ContiguousReaderError::DecompressFailed(e));
                }

                Cow::Owned(decompressed)
            } else {
                Cow::Borrowed(data)
            },
            offset: 0,
        })
    }
}

impl<'pup> EntryReader for NonBlockedReader<'pup> {}

impl<'pup> Seek for NonBlockedReader<'pup> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.offset = match pos {
            SeekFrom::Start(v) => min(self.data.len() as u64, v),
            SeekFrom::End(v) => {
                if v >= 0 {
                    self.data.len() as u64
                } else if v == i64::MIN {
                    return Err(ErrorKind::InvalidInput.into());
                } else {
                    let v = v.unsigned_abs() as usize;

                    match self.data.len().checked_sub(v) {
                        Some(v) => v as u64,
                        None => return Err(ErrorKind::InvalidInput.into()),
                    }
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    match self.offset.checked_add(v as u64) {
                        Some(v) => min(self.data.len() as u64, v),
                        None => self.data.len() as u64,
                    }
                } else if v == i64::MIN {
                    return Err(ErrorKind::InvalidInput.into());
                } else {
                    let v = v.unsigned_abs();

                    match self.offset.checked_sub(v) {
                        Some(v) => v,
                        None => return Err(ErrorKind::InvalidInput.into()),
                    }
                }
            }
        };

        Ok(self.offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.offset = 0;
        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.offset)
    }
}

impl<'pup> Read for NonBlockedReader<'pup> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let offset = self.offset as usize;

        if buf.is_empty() || offset == self.data.len() {
            return Ok(0);
        }

        let src = &self.data[offset..];
        let amount = min(src.len(), buf.len());
        let dst = buf.as_mut_ptr();

        unsafe { dst.copy_from_nonoverlapping(src.as_ptr(), amount) };
        self.offset += amount as u64;

        Ok(amount)
    }
}

pub(super) struct BlockedReader<'pup> {
    entry: &'pup Entry,
    pup: &'pup [u8],
    blocks: Vec<Block>,
    block_size: u32, // Size of each block, in uncompressed form.
    tail_size: u32,
    current_block: Vec<u8>,
    offset: usize, // Virtual offset into uncompressed data.
}

impl<'pup> BlockedReader<'pup> {
    pub fn new(
        entry: &'pup Entry,
        table: &'pup Entry,
        pup: &'pup [u8],
    ) -> Result<Self, BlockedReaderError> {
        let block_size = entry.block_size();
        let block_count = (block_size as u64 + entry.uncompressed_size() - 1) / block_size as u64;
        let tail_size = entry.uncompressed_size() % block_size as u64;

        // Create table reader.
        let table_offset = table.offset() as usize;
        let table_data = &pup[table_offset..(table_offset + table.compressed_size() as usize)];
        let mut table_reader: Box<dyn Read + 'pup> = if table.is_compressed() {
            Box::new(ZlibDecoder::new(table_data))
        } else {
            Box::new(table_data)
        };

        // Not sure what is this data. Maybe signature?
        for _ in 0..block_count {
            let mut buf: [u8; 32] = unsafe { uninit() };

            if let Err(e) = table_reader.read_exact(&mut buf) {
                return Err(BlockedReaderError::ReadTableFailed(e));
            }
        }

        // Read table.
        let mut blocks: Vec<Block> = Vec::with_capacity(block_count as _);

        for _ in 0..block_count {
            let data: [u8; 8] = match util::io::read_array(&mut table_reader) {
                Ok(v) => v,
                Err(e) => return Err(BlockedReaderError::ReadTableFailed(e)),
            };

            let data = data.as_ptr();
            let offset = unsafe { read_u32_le(data, 0) };
            let size = unsafe { read_u32_le(data, 4) };

            blocks.push(Block { offset, size })
        }

        Ok(Self {
            entry,
            pup,
            blocks,
            block_size,
            tail_size: if tail_size == 0 {
                block_size
            } else {
                tail_size as u32
            },
            current_block: Vec::new(),
            offset: 0,
        })
    }
}

impl<'pup> EntryReader for BlockedReader<'pup> {}

impl<'pup> Seek for BlockedReader<'pup> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let offset = match pos {
            SeekFrom::Start(v) => min(self.entry.uncompressed_size(), v),
            SeekFrom::End(v) => {
                if v >= 0 {
                    self.entry.uncompressed_size()
                } else if v == i64::MIN {
                    return Err(ErrorKind::InvalidInput.into());
                } else {
                    match self.entry.uncompressed_size().checked_sub(v.unsigned_abs()) {
                        Some(v) => v,
                        None => return Err(ErrorKind::InvalidInput.into()),
                    }
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    match self.offset.checked_add(v as usize) {
                        Some(v) => min(self.entry.uncompressed_size(), v as u64),
                        None => self.entry.uncompressed_size(),
                    }
                } else if v == i64::MIN {
                    return Err(ErrorKind::InvalidInput.into());
                } else {
                    match self.offset.checked_sub(v.unsigned_abs() as usize) {
                        Some(v) => v as u64,
                        None => return Err(ErrorKind::InvalidInput.into()),
                    }
                }
            }
        };

        if offset as usize != self.offset {
            self.current_block.clear();
            self.offset = offset as usize;
        }

        Ok(offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.current_block.clear();
        self.offset = 0;
        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.offset as u64)
    }
}

impl<'pup> Read for BlockedReader<'pup> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.offset == self.entry.uncompressed_size() as usize {
            return Ok(0);
        }

        let dst = buf.as_mut_ptr();
        let mut copied = 0usize;

        loop {
            // Load current block.
            if self.current_block.is_empty() {
                // Get block info.
                let block_index = self.offset / self.block_size as usize;
                let block_offset = self.offset % self.block_size as usize;
                let block = &self.blocks[block_index];

                // Get compressed data.
                let data_offset = match self.entry.offset().checked_add(block.offset as u64) {
                    Some(v) => v as usize,
                    None => {
                        return Err(std::io::Error::new(
                            ErrorKind::Other,
                            format!("block #{} has invalid data offset", block_index),
                        ));
                    }
                };

                let unpadded_size = (block.size & !0xf) - (block.size & 0xf);
                let (size, compressed) = if unpadded_size != self.block_size {
                    if block_index + 1 != self.blocks.len() || self.tail_size != block.size {
                        (unpadded_size, true)
                    } else {
                        (block.size, false)
                    }
                } else {
                    (self.block_size, false)
                };

                let data = match self.pup.get(data_offset..(data_offset + size as usize)) {
                    Some(v) => v,
                    None => {
                        return Err(std::io::Error::new(
                            ErrorKind::Other,
                            format!("block #{} has invalid data offset", block_index),
                        ));
                    }
                };

                // Decompress data.
                if compressed {
                    let mut decoder = ZlibDecoder::new(data);

                    if let Err(_) = decoder.read_to_end(&mut self.current_block) {
                        return Err(std::io::Error::new(
                            ErrorKind::Other,
                            format!("invalid data on block #{}", block_index),
                        ));
                    };
                } else {
                    self.current_block.extend_from_slice(data);
                }

                // Discard all data before current offset.
                self.current_block.drain(..block_offset);
            }

            // Copy data.
            let dst = unsafe { dst.add(copied) };
            let need = buf.len() - copied;
            let amount = min(self.current_block.len(), need);
            let src = self.current_block.drain(..amount);

            unsafe { dst.copy_from_nonoverlapping(src.as_slice().as_ptr(), amount) };
            self.offset += amount;
            copied += amount;

            drop(src);

            // Check if completed.
            if copied == buf.len() || self.offset == self.entry.uncompressed_size() as usize {
                break Ok(copied);
            } else if self.offset > self.entry.uncompressed_size() as usize {
                panic!("Offset advanced past the end of expected size.");
            }
        }
    }
}

struct Block {
    offset: u32,
    size: u32,
}

#[derive(Debug)]
pub enum ContiguousReaderError {
    InvalidOffset,
    DecompressFailed(std::io::Error),
}

impl Error for ContiguousReaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DecompressFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for ContiguousReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOffset => f.write_str("entry has invalid data offset"),
            Self::DecompressFailed(_) => f.write_str("cannot decompress data"),
        }
    }
}

#[derive(Debug)]
pub enum BlockedReaderError {
    ReadTableFailed(std::io::Error),
}

impl Error for BlockedReaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadTableFailed(e) => Some(e),
        }
    }
}

impl Display for BlockedReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadTableFailed(_) => f.write_str("cannot read data table"),
        }
    }
}
