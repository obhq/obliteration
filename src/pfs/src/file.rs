use crate::inode::Inode;
use crate::Image;
use std::cmp::min;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom};
use std::sync::Arc;

pub struct File<'pfs, 'image> {
    image: Arc<dyn Image + 'image>,
    inode: &'pfs Inode<'image>,
    occupied_blocks: Vec<u32>,
    current_offset: u64,
    current_block: Vec<u8>,
}

impl<'pfs, 'image> File<'pfs, 'image> {
    pub(crate) fn new(image: Arc<dyn Image + 'image>, inode: &'pfs Inode<'image>) -> Self {
        Self {
            image,
            inode,
            occupied_blocks: Vec::new(),
            current_offset: 0,
            current_block: Vec::new(),
        }
    }

    pub fn len(&self) -> u64 {
        self.inode.size()
    }

    pub fn is_compressed(&self) -> bool {
        self.inode.flags().is_compressed()
    }
}

impl<'pfs, 'image> Seek for File<'pfs, 'image> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        // Calculate new offset.
        let offset = match pos {
            SeekFrom::Start(v) => min(self.len(), v),
            SeekFrom::End(v) => {
                if v >= 0 {
                    self.len()
                } else {
                    let v = v.abs() as u64;

                    if v > self.len() {
                        return Err(Error::from(ErrorKind::InvalidInput));
                    }

                    self.len() - v
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    min(self.len(), self.current_offset + (v as u64))
                } else {
                    let v = v.abs() as u64;

                    if v > self.current_offset {
                        return Err(Error::from(ErrorKind::InvalidInput));
                    }

                    self.current_offset - v
                }
            }
        };

        // Update offset.
        if offset != self.current_offset {
            self.current_offset = offset;
            self.current_block.clear();
        }

        Ok(self.current_offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.current_offset = 0;
        self.current_block.clear();
        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.current_offset)
    }
}

impl<'pfs, 'image> Read for File<'pfs, 'image> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.current_offset == self.len() {
            return Ok(0);
        }

        // Load occupied blocks.
        if self.occupied_blocks.is_empty() {
            self.occupied_blocks = match self.inode.load_blocks() {
                Ok(v) => v,
                Err(e) => return Err(Error::new(ErrorKind::Other, e)),
            };
        }

        // Copy data.
        let block_size = self.image.header().block_size();
        let mut copied = 0usize;

        loop {
            // Load block for current offset.
            if self.current_block.is_empty() {
                // Get block number.
                let block_index = self.current_offset / (block_size as u64);
                let block_num = match self.occupied_blocks.get(block_index as usize) {
                    Some(&v) => v,
                    None => {
                        break Err(Error::new(
                            ErrorKind::Other,
                            format!("block #{} is not available", block_index),
                        ));
                    }
                };

                // Check if this is a last block.
                let total = block_index * (block_size as u64) + (block_size as u64);
                let read_amount = if total > self.len() {
                    // Both total and len never be zero.
                    (block_size as u64) - (total - self.len())
                } else {
                    block_size as u64
                };

                // Allocate buffer.
                self.current_block.reserve(read_amount as usize);
                unsafe { self.current_block.set_len(read_amount as usize) };

                // Load block data.
                let offset = (block_num as u64) * (block_size as u64);

                if let Err(e) = self.image.read(offset as usize, &mut self.current_block) {
                    break Err(Error::new(ErrorKind::Other, e));
                }
            }

            // Get a window into current block from current offset.
            let offset = self.current_offset % (block_size as u64);
            let src = &self.current_block[(offset as usize)..];

            // Copy the window to output buffer.
            let dst = unsafe { buf.as_mut_ptr().offset(copied as _) };
            let amount = min(src.len(), buf.len() - copied) as u32;

            unsafe { dst.copy_from_nonoverlapping(src.as_ptr(), amount as usize) };
            copied += amount as usize;

            // Advance current offset.
            self.current_offset += amount as u64;

            if self.current_offset % (block_size as u64) == 0 {
                self.current_block.clear();
            }

            // Check if completed.
            if copied == buf.len() || self.current_offset == self.len() {
                break Ok(copied);
            }
        }
    }
}
