use flate2::read::DeflateDecoder;
use flate2::{Decompress, FlushDecompress};
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{ErrorKind, IoSliceMut, Read};
use util::mem::{new_buffer, read_u32_le, read_u64_le, uninit};

pub struct Entry {
    flags: u32,
    offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
}

impl Entry {
    pub(super) const RAW_SIZE: usize = 32;

    pub(super) fn read(data: *const u8) -> Self {
        let flags = read_u32_le(data, 0);
        let offset = read_u64_le(data, 8);
        let compressed_size = read_u64_le(data, 16);
        let uncompressed_size = read_u64_le(data, 24);

        Self {
            flags,
            offset,
            compressed_size,
            uncompressed_size,
        }
    }

    pub fn id(&self) -> u16 {
        (self.flags >> 20) as u16
    }

    pub fn is_compressed(&self) -> bool {
        (self.flags & 8) != 0
    }

    pub fn is_blocked(&self) -> bool {
        (self.flags & 0x800) != 0
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    pub fn uncompressed_size(&self) -> u64 {
        self.uncompressed_size
    }
}

pub(super) struct ContiguousReader<'pup> {
    reader: Box<dyn Read + 'pup>,
}

impl<'pup> ContiguousReader<'pup> {
    pub fn new(entry: &'pup Entry, pup: &'pup [u8]) -> Self {
        let offset = entry.offset() as usize;
        let data = &pup[offset..(offset + entry.compressed_size() as usize)];

        Self {
            reader: if entry.is_compressed() {
                Box::new(DeflateDecoder::new(data))
            } else {
                Box::new(data)
            },
        }
    }
}

impl<'pup> Read for ContiguousReader<'pup> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.reader.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.reader.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.reader.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.reader.read_exact(buf)
    }
}

pub(super) struct BlockedReader<'pup> {
    entry: &'pup Entry,
    pup: &'pup [u8],
    blocks: Vec<Block>,
    block_size: u32,
    tail_size: u32,
    block_count: u64,
    next_block: u64,
    current_block: Vec<u8>,
}

impl<'pup> BlockedReader<'pup> {
    pub fn new(
        entry: &'pup Entry,
        table: &'pup Entry,
        pup: &'pup [u8],
    ) -> Result<Self, ReaderError> {
        if ((entry.id() | 0x100) & 0xf00) == 0xf00 {
            todo!()
        }

        let block_size = 1u32 << (((entry.flags() & 0xf000) >> 12) + 12); // maximum to shift is 27.
        let block_count = (block_size as u64 + entry.uncompressed_size() - 1) / block_size as u64;
        let tail_size = entry.uncompressed_size() % block_size as u64;
        let blocks: Vec<Block> = if entry.is_compressed() {
            let table_offset = table.offset() as usize;
            let table_data = &pup[table_offset..(table_offset + table.compressed_size() as usize)];
            let mut blocks: Vec<Block> = Vec::with_capacity(block_count as _);
            let mut table_reader: Box<dyn Read + 'pup> = if table.is_compressed() {
                Box::new(DeflateDecoder::new(table_data))
            } else {
                Box::new(table_data)
            };

            // Not sure what is this data. Maybe signature?
            for _ in 0..block_count {
                let mut buf: [u8; 32] = uninit();

                if let Err(e) = table_reader.read_exact(&mut buf) {
                    return Err(ReaderError::ReadTableFailed(e));
                }
            }

            for _ in 0..block_count {
                let data: [u8; 8] = match util::io::read_array(&mut table_reader) {
                    Ok(v) => v,
                    Err(e) => return Err(ReaderError::ReadTableFailed(e)),
                };

                let data = data.as_ptr();
                let offset = read_u32_le(data, 0);
                let size = read_u32_le(data, 4);

                blocks.push(Block { offset, size })
            }

            blocks
        } else {
            Vec::new()
        };

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
            block_count,
            next_block: 0,
            current_block: Vec::new(),
        })
    }
}

impl<'pup> Read for BlockedReader<'pup> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.next_block == self.block_count && self.current_block.is_empty() {
            return Ok(0);
        }

        let dst = buf.as_mut_ptr();
        let mut copied = 0usize;

        loop {
            let dst = unsafe { dst.offset(copied as _) };
            let need = buf.len() - copied;
            let amount = min(self.current_block.len(), need);
            let src = self.current_block.drain(..amount);

            unsafe { dst.copy_from_nonoverlapping(src.as_slice().as_ptr(), amount) };
            copied += amount;

            drop(src);

            if copied == buf.len() {
                break Ok(copied);
            }

            // current_block will be always empty once we reached here.
            let (block, compressed) = if self.entry.is_compressed() {
                let block = &self.blocks[self.next_block as usize];
                let unpadded_size = (block.size & !0xf) - (block.size & 0xf);
                let offset = (self.entry.offset() + block.offset as u64) as usize;
                let (size, compressed) = if unpadded_size != self.block_size {
                    if self.next_block + 1 != self.block_count || self.tail_size != block.size {
                        ((block.size & !0xf) - (block.size & 0xf), true)
                    } else {
                        (block.size, false)
                    }
                } else {
                    (self.block_size, false)
                };

                (&self.pup[offset..(offset + size as usize)], compressed)
            } else {
                let offset =
                    (self.entry.offset() + self.block_size as u64 * self.next_block) as usize;
                let remaining =
                    self.entry.compressed_size() - self.block_size as u64 * self.next_block;
                let size = min(remaining, self.block_size as u64) as usize;

                (&self.pup[offset..(offset + size)], false)
            };

            if compressed {
                let mut deflate = Decompress::new(false);
                let mut decompressed: Vec<u8> = new_buffer(block.len());
                let status =
                    match deflate.decompress(block, &mut decompressed, FlushDecompress::Finish) {
                        Ok(v) => v,
                        Err(e) => return Err(std::io::Error::new(ErrorKind::Other, e)),
                    };

                if status != flate2::Status::StreamEnd {
                    return Err(std::io::Error::new(
                        ErrorKind::Other,
                        format!("invalid data on block #{}", self.next_block),
                    ));
                }

                unsafe { decompressed.set_len(deflate.total_out() as _) };
                self.current_block.extend(decompressed);
            } else {
                self.current_block.extend_from_slice(block);
            }

            self.next_block += 1;
        }
    }
}

struct Block {
    offset: u32,
    size: u32,
}

#[derive(Debug)]
pub enum ReaderError {
    ReadTableFailed(std::io::Error),
}

impl Error for ReaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadTableFailed(e) => Some(e),
        }
    }
}

impl Display for ReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadTableFailed(_) => f.write_str("cannot read table"),
        }
    }
}
