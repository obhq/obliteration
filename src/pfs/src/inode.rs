use crate::header::Mode;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Read;
use util::mem::{read_array, read_u32_le, uninit};

pub struct Inode {
    blocks: u32,
    direct_blocks: [u32; 12],
    direct_sigs: [Option<[u8; 32]>; 12],
    indirect_blocks: [u32; 5],
    indirect_signs: [Option<[u8; 32]>; 5],
}

impl Inode {
    pub fn read_unsigned<F: Read>(from: &mut F) -> Result<Self, ReadError> {
        // Read common fields.
        let raw: [u8; 168] = Self::read_raw(from)?;
        let mut ptr = raw.as_ptr();
        let mut inode = Self::read_commons(ptr);

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

    pub fn read_signed<F: Read>(from: &mut F) -> Result<Self, ReadError> {
        // Read common fields.
        let raw: [u8; 712] = Self::read_raw(from)?;
        let mut ptr = raw.as_ptr();
        let mut inode = Self::read_commons(ptr);

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

    pub fn direct_blocks(&self) -> &[u32; 12] {
        &self.direct_blocks
    }

    pub fn indirect_blocks(&self) -> &[u32; 5] {
        &self.indirect_blocks
    }

    pub fn block_count(&self) -> usize {
        self.blocks as _
    }

    fn read_raw<const L: usize, F: Read>(from: &mut F) -> Result<[u8; L], ReadError> {
        let mut raw: [u8; L] = uninit();

        if let Err(e) = from.read_exact(&mut raw) {
            return Err(if e.kind() == std::io::ErrorKind::UnexpectedEof {
                ReadError::TooSmall
            } else {
                ReadError::IoFailed(e)
            });
        }

        Ok(raw)
    }

    fn read_commons(raw: *const u8) -> Self {
        let blocks = read_u32_le(raw, 0x60);

        Self {
            blocks,
            direct_blocks: [0; 12],
            direct_sigs: [None; 12],
            indirect_blocks: [0; 5],
            indirect_signs: [None; 5],
        }
    }
}

pub struct BlockPointers<'raw> {
    mode: Mode,
    next: &'raw [u8],
}

impl<'raw> BlockPointers<'raw> {
    pub fn new(mode: Mode, raw: &'raw [u8]) -> Self {
        if mode.is_64bits() {
            panic!("64-bits inode is not supported");
        }

        Self { mode, next: raw }
    }
}

impl<'raw> Iterator for BlockPointers<'raw> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.mode.is_signed() {
            let value = match self.next.get(..36) {
                Some(v) => read_u32_le(v.as_ptr(), 32),
                None => return None,
            };

            self.next = &self.next[36..];

            Some(value as usize)
        } else {
            let value = match self.next.get(..4) {
                Some(v) => read_u32_le(v.as_ptr(), 0),
                None => return None,
            };

            self.next = &self.next[4..];

            Some(value as usize)
        }
    }
}

#[derive(Debug)]
pub enum ReadError {
    IoFailed(std::io::Error),
    TooSmall,
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IoFailed(_) => f.write_str("I/O failed"),
            Self::TooSmall => f.write_str("data too small"),
        }
    }
}
