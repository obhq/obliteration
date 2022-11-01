use flate2::read::ZlibDecoder;
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use util::mem::{new_buffer, read_array, read_u32_le, read_u64_le, uninit};
use util::slice::as_mut_bytes;

// FIXME: Refactor the whole implementation of this since a lot of logic does not make sense.
pub struct Reader<F: Read + Seek> {
    file: F,
    block_size: u32,
    original_block_size: u64,
    compressed_blocks: Vec<u64>, // Original block number to compressed block offset.
    original_size: u64,
    current_offset: u64,
    current_block: Vec<u8>,
}

impl<F: Read + Seek> Reader<F> {
    pub fn open(mut file: F) -> Result<Self, OpenError> {
        // Seek to beginning.
        if let Err(e) = file.rewind() {
            return Err(OpenError::SeekFailed(0, e));
        }

        // Check header.
        let mut hdr: [u8; 48] = uninit();

        if let Err(e) = file.read_exact(&mut hdr) {
            return Err(if e.kind() == ErrorKind::UnexpectedEof {
                OpenError::TooSmall
            } else {
                OpenError::IoFailed(e)
            });
        }

        let hdr = hdr.as_ptr();
        let magic: [u8; 4] = read_array(hdr, 0);

        if &magic != b"PFSC" {
            return Err(OpenError::InvalidMagic);
        }

        // Read header.
        let block_size = read_u32_le(hdr, 0x0c); // BlockSz
        let original_block_size = read_u64_le(hdr, 0x10); // BlockSz2
        let block_offsets = read_u64_le(hdr, 0x18); // BlockOffsets
        let original_size = read_u64_le(hdr, 0x28); // DataLength

        // Read block offsets.
        if let Err(e) = file.seek(SeekFrom::Start(block_offsets)) {
            return Err(OpenError::SeekFailed(block_offsets, e));
        }

        let original_block_count = original_size / original_block_size + 1;
        let mut compressed_blocks: Vec<u64> = new_buffer(original_block_count as usize);

        if let Err(e) = file.read_exact(as_mut_bytes(&mut compressed_blocks)) {
            return Err(OpenError::ReadBlockMappingFailed(e));
        }

        Ok(Self {
            file,
            block_size,
            original_block_size,
            compressed_blocks,
            original_size,
            current_offset: 0,
            current_block: Vec::new(),
        })
    }

    pub fn len(&self) -> u64 {
        self.original_size
    }

    fn read_compressed_block(&mut self, num: u64) -> std::io::Result<()> {
        // Get end offset.
        let end = match self.compressed_blocks.get(num as usize + 1) {
            Some(v) => v,
            None => return Err(std::io::Error::from(ErrorKind::InvalidInput)),
        };

        // Get start offset and compressed size.
        let offset = self.compressed_blocks[num as usize];
        let size = end - offset;

        // Allocate buffer.
        self.current_block.reserve(self.block_size as usize);
        unsafe { self.current_block.set_len(self.block_size as usize) };

        let buf = self.current_block.as_mut_slice();

        // Check if block compressed.
        if size == self.original_block_size {
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(buf)?;
        } else if size > self.original_block_size {
            buf.fill(0);
        } else {
            // Read compressed.
            let mut compressed = new_buffer(size as usize);

            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(&mut compressed)?;

            // Decompress.
            let mut deflate = ZlibDecoder::new(buf);
            let status = match deflate.read_to_end(&mut compressed) {
                Ok(v) => v,
                Err(e) => return Err(std::io::Error::new(ErrorKind::Other, e)),
            };

            if status != flate2::Status::StreamEnd || deflate.total_out() as usize != buf.len() {
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    format!("invalid data on block #{}", num),
                ));
            }
        }

        Ok(())
    }
}

impl<F: Read + Seek> Read for Reader<F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.current_offset == self.original_size {
            return Ok(0);
        }

        // Copy data.
        let mut copied = 0usize;

        loop {
            // Load block for current offset.
            if self.current_block.is_empty() {
                // Load block data.
                let block_index = self.current_offset / (self.block_size as u64);

                self.read_compressed_block(block_index)?;

                // Check if this is a last block.
                let total = block_index * (self.block_size as u64) + (self.block_size as u64);

                if total > self.original_size {
                    let need = (self.block_size as u64) - (total - self.original_size);
                    self.current_block.truncate(need as usize);
                };
            }

            // Get a window into current block from current offset.
            let offset = self.current_offset % (self.block_size as u64);
            let src = &self.current_block[(offset as usize)..];

            // Copy the window to output buffer.
            let dst = unsafe { buf.as_mut_ptr().offset(copied as _) };
            let amount = min(src.len(), buf.len() - copied);

            unsafe { dst.copy_from_nonoverlapping(src.as_ptr(), amount as usize) };
            copied += amount as usize;

            // Advance current offset.
            self.current_offset += amount as u64;

            if self.current_offset % (self.block_size as u64) == 0 {
                self.current_block.clear();
            }

            // Check if completed.
            if copied == buf.len() || self.current_offset == self.original_size {
                break Ok(copied);
            }
        }
    }
}

#[derive(Debug)]
pub enum OpenError {
    SeekFailed(u64, std::io::Error),
    IoFailed(std::io::Error),
    TooSmall,
    InvalidMagic,
    ReadBlockMappingFailed(std::io::Error),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SeekFailed(_, e) => Some(e),
            Self::IoFailed(e) => Some(e),
            Self::ReadBlockMappingFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::SeekFailed(p, _) => write!(f, "cannot seek to offset {}", p),
            Self::IoFailed(_) => f.write_str("I/O failed"),
            Self::TooSmall => f.write_str("data too small"),
            Self::InvalidMagic => f.write_str("invalid magic"),
            Self::ReadBlockMappingFailed(_) => f.write_str("cannot read block mapping"),
        }
    }
}
