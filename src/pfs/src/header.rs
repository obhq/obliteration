use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::{read_array, read_u16_le, read_u32_le, read_u64_le};

// https://www.psdevwiki.com/ps4/PFS#Header.2FSuperblock
pub struct Header {
    mode: Mode,
    blocksz: u32,
    ndinode: u64,
    ndinodeblock: u32,
    superroot_ino: u64,
    key_seed: [u8; 16],
}

impl Header {
    pub fn read(image: &[u8]) -> Result<Self, ReadError> {
        if image.len() < 0x380 {
            return Err(ReadError::TooSmall);
        }

        let hdr = image.as_ptr();

        // Check version.
        let version = read_u64_le(hdr, 0x00);

        if version != 1 {
            return Err(ReadError::InvalidVersion);
        }

        // Check format.
        let format = read_u64_le(hdr, 0x08);

        if format != 20130315 {
            return Err(ReadError::InvalidFormat);
        }

        // Read fields.
        let mode = Mode(read_u16_le(hdr, 0x1c));
        let blocksz = read_u32_le(hdr, 0x20);
        let ndinode = read_u64_le(hdr, 0x30);
        let ndinodeblock = read_u64_le(hdr, 0x40);
        let superroot_ino = read_u64_le(hdr, 0x48);
        let key_seed = read_array(hdr, 0x370);

        // Usually block will be references by u32. Not sure why ndinodeblock is 64-bits. Design
        // flaws?
        if ndinodeblock > (u32::MAX as u64) {
            return Err(ReadError::TooManyInodeBlocks);
        }

        Ok(Self {
            mode,
            blocksz,
            ndinode,
            ndinodeblock: ndinodeblock as u32,
            superroot_ino,
            key_seed,
        })
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn block_size(&self) -> u32 {
        self.blocksz
    }

    /// Gets a number of total inodes.
    pub fn inode_count(&self) -> usize {
        self.ndinode as _
    }

    /// Gets a number of blocks containing inode (not a number of inode).
    pub fn inode_block_count(&self) -> u32 {
        self.ndinodeblock
    }

    pub fn super_root_inode(&self) -> usize {
        self.superroot_ino as _
    }

    pub fn key_seed(&self) -> &[u8; 16] {
        &self.key_seed
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Mode(u16);

impl Mode {
    pub fn is_signed(&self) -> bool {
        self.0 & 0x1 != 0
    }

    pub fn is_64bits(&self) -> bool {
        self.0 & 0x2 != 0
    }

    pub fn is_encrypted(&self) -> bool {
        self.0 & 0x4 != 0
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:x}", self.0)?;

        let mut op = false;
        let mut first = true;
        let mut flag = |name: &str| -> std::fmt::Result {
            if !op {
                f.write_str(" (")?;
                op = true;
            }

            if !first {
                f.write_str(", ")?;
            }

            f.write_str(name)?;
            first = false;

            Ok(())
        };

        if self.is_signed() {
            flag("signed")?;
        }

        if self.is_64bits() {
            flag("64-bits")?;
        }

        if self.is_encrypted() {
            flag("encrypted")?;
        }

        if op {
            f.write_str(")")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ReadError {
    TooSmall,
    InvalidVersion,
    InvalidFormat,
    TooManyInodeBlocks,
}

impl Error for ReadError {}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::TooSmall => f.write_str("data too small"),
            Self::InvalidVersion => f.write_str("invalid version"),
            Self::InvalidFormat => f.write_str("invalid format"),
            Self::TooManyInodeBlocks => f.write_str("too many blocks for inodes"),
        }
    }
}
