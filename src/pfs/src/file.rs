use crate::inode::Inode;
use crate::Pfs;
use std::cmp::min;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom};
use std::ops::DerefMut;
use std::sync::Arc;

/// Represents a file in the PFS.
pub struct File<'a> {
    pfs: Arc<Pfs<'a>>,
    inode: usize,
    occupied_blocks: Vec<u32>,
    current_offset: u64,
    current_block: Vec<u8>,
}

impl<'a> File<'a> {
    pub(crate) fn new(pfs: Arc<Pfs<'a>>, inode: usize) -> Self {
        Self {
            pfs,
            inode,
            occupied_blocks: Vec::new(),
            current_offset: 0,
            current_block: Vec::new(),
        }
    }

    pub fn len(&self) -> Option<u64> {
        self.inode().map(|i| i.size())
    }

    pub fn is_compressed(&self) -> Option<bool> {
        self.inode().map(|i| i.flags().is_compressed())
    }

    fn inode(&self) -> Option<&Inode> {
        self.pfs.inodes.get(self.inode)
    }
}

impl<'a> Clone for File<'a> {
    fn clone(&self) -> Self {
        Self {
            pfs: self.pfs.clone(),
            inode: self.inode,
            occupied_blocks: Vec::new(),
            current_offset: self.current_offset,
            current_block: Vec::new(),
        }
    }
}

impl<'a> Seek for File<'a> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        // Get inode.
        let inode = match self.pfs.inodes.get(self.inode) {
            Some(v) => v,
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("inode #{} does not exists", self.inode),
                ))
            }
        };

        // Calculate new offset.
        let offset = match pos {
            SeekFrom::Start(v) => min(inode.size(), v),
            SeekFrom::End(v) => {
                if v >= 0 {
                    inode.size()
                } else {
                    let v = v.unsigned_abs();

                    if v > inode.size() {
                        return Err(Error::from(ErrorKind::InvalidInput));
                    }

                    inode.size() - v
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    min(inode.size(), self.current_offset + (v as u64))
                } else {
                    let v = v.unsigned_abs();

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

impl<'a> Read for File<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Get inode.
        let inode = match self.pfs.inodes.get(self.inode) {
            Some(v) => v,
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("inode #{} does not exists", self.inode),
                ))
            }
        };

        // Check if we need to do the actual read.
        if buf.is_empty() || self.current_offset == inode.size() {
            return Ok(0);
        }

        // Load occupied blocks.
        let mut image = self.pfs.image.lock().unwrap();
        let image = image.deref_mut();

        if self.occupied_blocks.is_empty() {
            self.occupied_blocks = match inode.load_blocks(image.as_mut()) {
                Ok(v) => v,
                Err(e) => return Err(Error::new(ErrorKind::Other, e)),
            };
        }

        // Copy data.
        let block_size = image.header().block_size();
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
                let read_amount = if total > inode.size() {
                    // Both total and len never be zero.
                    (block_size as u64) - (total - inode.size())
                } else {
                    block_size as u64
                };

                // Allocate buffer.
                self.current_block.reserve(read_amount as usize);
                unsafe { self.current_block.set_len(read_amount as usize) };

                // Seek to block.
                let offset = (block_num as u64) * (block_size as u64);

                match image.seek(SeekFrom::Start(offset)) {
                    Ok(v) => {
                        if v != offset {
                            return Err(Error::new(
                                ErrorKind::Other,
                                format!("block #{} does not exists", block_num),
                            ));
                        }
                    }
                    Err(e) => return Err(e),
                }

                // Load block data.
                image.read_exact(&mut self.current_block)?;
            }

            // Get a window into current block from current offset.
            let offset = self.current_offset % (block_size as u64);
            let src = &self.current_block[(offset as usize)..];

            // Copy the window to output buffer.
            let dst = unsafe { buf.as_mut_ptr().add(copied) };
            let amount = min(src.len(), buf.len() - copied) as u32;

            unsafe { dst.copy_from_nonoverlapping(src.as_ptr(), amount as usize) };
            copied += amount as usize;

            // Advance current offset.
            self.current_offset += amount as u64;

            if self.current_offset % (block_size as u64) == 0 {
                self.current_block.clear();
            }

            // Check if completed.
            if copied == buf.len() || self.current_offset == inode.size() {
                break Ok(copied);
            }
        }
    }
}
