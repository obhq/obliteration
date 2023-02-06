use crate::Image;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, SeekFrom};
use thiserror::Error;
use util::mem::{new_buffer, read_array, read_u16_le, read_u32_le, read_u64_le, uninit};

/// Contains information for an inode.
pub(crate) struct Inode {
    index: usize,
    mode: u16,
    flags: InodeFlags,
    size: u64,
    decompressed_size: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    birthtime: u64,
    mtimensec: u32,
    atimensec: u32,
    ctimensec: u32,
    birthnsec: u32,
    uid: u32,
    gid: u32,
    blocks: u32,
    direct_blocks: [u32; 12],
    direct_sigs: [Option<[u8; 32]>; 12],
    indirect_blocks: [u32; 5],
    indirect_signs: [Option<[u8; 32]>; 5],
    indirect_reader: fn(&mut &[u8]) -> Option<u32>,
}

impl Inode {
    pub(super) fn from_raw32_unsigned<R>(index: usize, raw: &mut R) -> Result<Self, FromRawError>
    where
        R: Read,
    {
        // Read common fields.
        let raw: [u8; 168] = Self::read_raw(raw)?;
        let mut ptr = raw.as_ptr();
        let mut inode = Self::read_common_fields(index, ptr, Self::read_indirect32_unsigned);

        // Read block pointers.
        ptr = unsafe { ptr.offset(0x64) };

        for i in 0..12 {
            inode.direct_blocks[i] = unsafe { read_u32_le(ptr, 0) };
            ptr = unsafe { ptr.offset(4) };
        }

        for i in 0..5 {
            inode.indirect_blocks[i] = unsafe { read_u32_le(ptr, 0) };
            ptr = unsafe { ptr.offset(4) };
        }

        Ok(inode)
    }

    pub(super) fn from_raw32_signed<R>(index: usize, raw: &mut R) -> Result<Self, FromRawError>
    where
        R: Read,
    {
        // Read common fields.
        let raw: [u8; 712] = Self::read_raw(raw)?;
        let mut ptr = raw.as_ptr();
        let mut inode = Self::read_common_fields(index, ptr, Self::read_indirect32_signed);

        // Read block pointers.
        ptr = unsafe { ptr.offset(0x64) };

        for i in 0..12 {
            inode.direct_sigs[i] = Some(unsafe { read_array(ptr, 0) });
            inode.direct_blocks[i] = unsafe { read_u32_le(ptr, 32) };
            ptr = unsafe { ptr.offset(36) };
        }

        for i in 0..5 {
            inode.indirect_signs[i] = Some(unsafe { read_array(ptr, 0) });
            inode.indirect_blocks[i] = unsafe { read_u32_le(ptr, 32) };
            ptr = unsafe { ptr.offset(36) };
        }

        Ok(inode)
    }

    pub fn mode(&self) -> u16 {
        self.mode
    }

    pub fn flags(&self) -> InodeFlags {
        self.flags
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn decompressed_size(&self) -> u64 {
        self.decompressed_size
    }

    pub fn atime(&self) -> u64 {
        self.atime
    }

    pub fn mtime(&self) -> u64 {
        self.mtime
    }

    pub fn ctime(&self) -> u64 {
        self.ctime
    }

    pub fn birthtime(&self) -> u64 {
        self.birthtime
    }

    pub fn mtimensec(&self) -> u32 {
        self.mtimensec
    }

    pub fn atimensec(&self) -> u32 {
        self.atimensec
    }

    pub fn ctimensec(&self) -> u32 {
        self.ctimensec
    }

    pub fn birthnsec(&self) -> u32 {
        self.birthnsec
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn gid(&self) -> u32 {
        self.gid
    }

    pub fn load_blocks(&self, image: &mut dyn Image) -> Result<Vec<u32>, LoadBlocksError> {
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
        let block_size = image.header().block_size();
        let offset = (block_num * block_size) as u64;

        match image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(LoadBlocksError::BlockNotExists(block_num));
                }
            }
            Err(e) => return Err(LoadBlocksError::SeekFailed(block_num, e)),
        }

        let mut block0 = unsafe { new_buffer(block_size as usize) };

        if let Err(e) = image.read_exact(&mut block0) {
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
        let offset = (block_num * block_size) as u64;

        match image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(LoadBlocksError::BlockNotExists(block_num));
                }
            }
            Err(e) => return Err(LoadBlocksError::SeekFailed(block_num, e)),
        }

        if let Err(e) = image.read_exact(&mut block0) {
            return Err(LoadBlocksError::ReadBlockFailed(block_num, e));
        }

        let mut block1 = unsafe { new_buffer(block_size as usize) };
        let mut data0 = block0.as_slice();

        while let Some(i) = (self.indirect_reader)(&mut data0) {
            let offset = (i * block_size) as u64;

            match image.seek(SeekFrom::Start(offset)) {
                Ok(v) => {
                    if v != offset {
                        return Err(LoadBlocksError::BlockNotExists(i));
                    }
                }
                Err(e) => return Err(LoadBlocksError::SeekFailed(i, e)),
            }

            if let Err(e) = image.read_exact(&mut block1) {
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
        let mut buf: [u8; L] = unsafe { uninit() };

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
        index: usize,
        raw: *const u8,
        indirect_reader: fn(&mut &[u8]) -> Option<u32>,
    ) -> Self {
        let mode = unsafe { read_u16_le(raw, 0x00) };
        let flags = InodeFlags(unsafe { read_u32_le(raw, 0x04) });
        let size = unsafe { read_u64_le(raw, 0x08) };
        let decompressed_size = unsafe { read_u64_le(raw, 0x10) };
        let atime = unsafe { read_u64_le(raw, 0x18) };
        let mtime = unsafe { read_u64_le(raw, 0x20) };
        let ctime = unsafe { read_u64_le(raw, 0x28) };
        let birthtime = unsafe { read_u64_le(raw, 0x30) };
        let mtimensec = unsafe { read_u32_le(raw, 0x38) };
        let atimensec = unsafe { read_u32_le(raw, 0x3c) };
        let ctimensec = unsafe { read_u32_le(raw, 0x40) };
        let birthnsec = unsafe { read_u32_le(raw, 0x44) };
        let uid = unsafe { read_u32_le(raw, 0x48) };
        let gid = unsafe { read_u32_le(raw, 0x4c) };
        let blocks = unsafe { read_u32_le(raw, 0x60) };

        Self {
            index,
            mode,
            flags,
            size,
            decompressed_size,
            atime,
            mtime,
            ctime,
            birthtime,
            mtimensec,
            atimensec,
            ctimensec,
            birthnsec,
            uid,
            gid,
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
            Some(v) => unsafe { read_u32_le(v.as_ptr(), 0) },
            None => return None,
        };

        *raw = &raw[4..];

        Some(value)
    }

    fn read_indirect32_signed(raw: &mut &[u8]) -> Option<u32> {
        let value = match raw.get(..36) {
            Some(v) => unsafe { read_u32_le(v.as_ptr(), 32) },
            None => return None,
        };

        *raw = &raw[36..];

        Some(value)
    }
}

/// Flags of the inode.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct InodeFlags(u32);

impl InodeFlags {
    pub fn is_compressed(self) -> bool {
        self.0 & 0x00000001 != 0
    }

    pub fn value(self) -> u32 {
        self.0
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

/// Errors for [`load_blocks()`][Inode::load_blocks()].
#[derive(Debug, Error)]
pub enum LoadBlocksError {
    #[error("cannot seek to block #{0}")]
    SeekFailed(u32, #[source] std::io::Error),

    #[error("block #{0} does not exists")]
    BlockNotExists(u32),

    #[error("cannot read block #{0}")]
    ReadBlockFailed(u32, #[source] std::io::Error),
}
