use crate::Image;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{new_buffer, read_array, read_u32_le, read_u64_le, uninit};

pub(crate) struct Inode<'image, 'raw_image> {
    image: &'image (dyn Image + 'raw_image),
    index: usize,
    flags: InodeFlags,
    size: u64,
    size_compressed: usize, // It look like this one is the size of entry when de-compressed.
    blocks: u32,
    direct_blocks: [u32; 12],
    direct_sigs: [Option<[u8; 32]>; 12],
    indirect_blocks: [u32; 5],
    indirect_signs: [Option<[u8; 32]>; 5],
    indirect_reader: fn(&mut &[u8]) -> Option<u32>,
}

impl<'image, 'raw_image> Inode<'image, 'raw_image> {
    pub(super) fn from_raw32_unsigned<R: Read>(
        image: &'image (dyn Image + 'raw_image),
        index: usize,
        raw: &mut R,
    ) -> Result<Self, FromRawError> {
        // Read common fields.
        let raw: [u8; 168] = Self::read_raw(raw)?;
        let mut ptr = raw.as_ptr();
        let mut inode = Self::read_common_fields(image, index, ptr, Self::read_indirect32_unsigned);

        // Read block pointers.
        ptr = unsafe { ptr.offset(0x64) };

        for i in 0..12 {
            inode.direct_blocks[i] = read_u32_le(ptr, 0);
            ptr = unsafe { ptr.offset(4) };
        }

        for i in 0..5 {
            inode.indirect_blocks[i] = read_u32_le(ptr, 0);
            ptr = unsafe { ptr.offset(4) };
        }

        Ok(inode)
    }

    pub(super) fn from_raw32_signed<R: Read>(
        image: &'image (dyn Image + 'raw_image),
        index: usize,
        raw: &mut R,
    ) -> Result<Self, FromRawError> {
        // Read common fields.
        let raw: [u8; 712] = Self::read_raw(raw)?;
        let mut ptr = raw.as_ptr();
        let mut inode = Self::read_common_fields(image, index, ptr, Self::read_indirect32_signed);

        // Read block pointers.
        ptr = unsafe { ptr.offset(0x64) };

        for i in 0..12 {
            inode.direct_sigs[i] = Some(read_array(ptr, 0));
            inode.direct_blocks[i] = read_u32_le(ptr, 32);
            ptr = unsafe { ptr.offset(36) };
        }

        for i in 0..5 {
            inode.indirect_signs[i] = Some(read_array(ptr, 0));
            inode.indirect_blocks[i] = read_u32_le(ptr, 32);
            ptr = unsafe { ptr.offset(36) };
        }

        Ok(inode)
    }

    pub fn flags(&self) -> InodeFlags {
        self.flags
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn compressed_size(&self) -> usize {
        self.size_compressed
    }

    pub fn load_blocks(&self) -> Result<Vec<u32>, LoadBlocksError> {
        // Check if inode use contiguous blocks.
        let mut blocks: Vec<u32> = Vec::with_capacity(self.blocks as usize);

        if blocks.len() == self.blocks as usize {
            // inode with zero block should not be possible but just in case for malformed image.
            return Ok(blocks);
        }

        if self.direct_blocks[1] == 0xffffffff {
            let start = self.direct_blocks[0];

            for block in start..(start + self.blocks) {
                blocks.push(block);
            }

            return Ok(blocks);
        }

        // Load direct pointers.
        for i in 0..12 {
            blocks.push(self.direct_blocks[i]);

            if blocks.len() == self.blocks as usize {
                return Ok(blocks);
            }
        }

        // FIXME: Refactor algorithm to read indirect blocks.
        // Load indirect 0.
        let block_num = self.indirect_blocks[0];
        let block_size = self.image.header().block_size();
        let offset = block_num * block_size;
        let mut block0 = new_buffer(block_size as usize);

        if let Err(e) = self.image.read(offset as usize, &mut block0) {
            return Err(LoadBlocksError::ReadBlockFailed(block_num, e));
        }

        let mut data = block0.as_slice();

        while let Some(i) = (self.indirect_reader)(&mut data) {
            blocks.push(i);

            if blocks.len() == self.blocks as usize {
                return Ok(blocks);
            }
        }

        // Load indirect 1.
        let block_num = self.indirect_blocks[1];
        let offset = block_num * block_size;

        if let Err(e) = self.image.read(offset as usize, &mut block0) {
            return Err(LoadBlocksError::ReadBlockFailed(block_num, e));
        }

        let mut block1 = new_buffer(block_size as usize);
        let mut data0 = block0.as_slice();

        while let Some(i) = (self.indirect_reader)(&mut data0) {
            if let Err(e) = self.image.read((i * block_size) as usize, &mut block1) {
                return Err(LoadBlocksError::ReadBlockFailed(i, e));
            }

            let mut data1 = block1.as_slice();

            while let Some(j) = (self.indirect_reader)(&mut data1) {
                blocks.push(j);

                if blocks.len() == self.blocks as usize {
                    return Ok(blocks);
                }
            }
        }

        panic!(
            "Data of inode #{} was spanned to indirect block #2, which we are not supported yet",
            self.index
        );
    }

    fn read_raw<const L: usize, R: Read>(raw: &mut R) -> Result<[u8; L], FromRawError> {
        let mut buf: [u8; L] = uninit();

        if let Err(e) = raw.read_exact(&mut buf) {
            return Err(if e.kind() == std::io::ErrorKind::UnexpectedEof {
                FromRawError::TooSmall
            } else {
                FromRawError::IoFailed(e)
            });
        }

        Ok(buf)
    }

    fn read_common_fields(
        image: &'image (dyn Image + 'raw_image),
        index: usize,
        raw: *const u8,
        indirect_reader: fn(&mut &[u8]) -> Option<u32>,
    ) -> Self {
        let flags = InodeFlags(read_u32_le(raw, 0x04));
        let size = read_u64_le(raw, 0x08);
        let size_compressed = read_u64_le(raw, 0x10) as usize;
        let blocks = read_u32_le(raw, 0x60);

        Self {
            image,
            index,
            flags,
            size,
            size_compressed,
            blocks,
            direct_blocks: [0; 12],
            direct_sigs: [None; 12],
            indirect_blocks: [0; 5],
            indirect_signs: [None; 5],
            indirect_reader,
        }
    }

    fn read_indirect32_unsigned(raw: &mut &[u8]) -> Option<u32> {
        let value = match raw.get(..4) {
            Some(v) => read_u32_le(v.as_ptr(), 0),
            None => return None,
        };

        *raw = &raw[4..];

        Some(value)
    }

    fn read_indirect32_signed(raw: &mut &[u8]) -> Option<u32> {
        let value = match raw.get(..36) {
            Some(v) => read_u32_le(v.as_ptr(), 32),
            None => return None,
        };

        *raw = &raw[36..];

        Some(value)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct InodeFlags(u32);

impl InodeFlags {
    pub fn is_compressed(self) -> bool {
        self.0 & 0x00000001 != 0
    }
}

#[derive(Debug)]
pub enum FromRawError {
    IoFailed(std::io::Error),
    TooSmall,
}

impl Error for FromRawError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for FromRawError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IoFailed(_) => f.write_str("I/O failed"),
            Self::TooSmall => f.write_str("data too small"),
        }
    }
}

#[derive(Debug)]
pub enum LoadBlocksError {
    ReadBlockFailed(u32, crate::ReadError),
}

impl Error for LoadBlocksError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBlockFailed(_, e) => Some(e),
        }
    }
}

impl Display for LoadBlocksError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
        }
    }
}
