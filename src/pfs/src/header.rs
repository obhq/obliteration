use byteorder::{ByteOrder, LE};
use std::fmt::{Display, Formatter};
use std::io::Read;
use thiserror::Error;

/// Contains PFS header.
///
/// See https://www.psdevwiki.com/ps4/PFS#Header.2FSuperblock for some basic information.
pub(crate) struct Header {
    mode: Mode,
    blocksz: u32,
    ndinode: u64,
    ndinodeblock: u32,
    superroot_ino: u64,
    key_seed: [u8; 16],
}

impl Header {
    pub(super) fn read<I: Read>(image: &mut I) -> Result<Self, ReadError> {
        // Read the whole header into the buffer.
        let mut hdr = [0u8; 0x380];

        if let Err(e) = image.read_exact(&mut hdr) {
            return Err(ReadError::IoFailed(e));
        }

        // Check version.
        let version = LE::read_u64(&hdr[0x00..]);

        if version != 1 {
            return Err(ReadError::InvalidVersion);
        }

        // Check format.
        let format = LE::read_u64(&hdr[0x08..]);

        if format != 20130315 {
            return Err(ReadError::InvalidFormat);
        }

        // Read fields.
        let mode = Mode(LE::read_u16(&hdr[0x1c..]));
        let blocksz = LE::read_u32(&hdr[0x20..]);
        let ndinode = LE::read_u64(&hdr[0x30..]);
        let ndinodeblock = LE::read_u64(&hdr[0x40..]);
        let superroot_ino = LE::read_u64(&hdr[0x48..]);
        let key_seed = &hdr[0x370..(0x370 + 16)];

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
            key_seed: key_seed.try_into().unwrap(),
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

/// Contains PFS flags.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct Mode(u16);

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

/// Errors for [read()][Header::read()].
#[derive(Debug, Error)]
pub enum ReadError {
    #[error("cannot read image")]
    IoFailed(#[source] std::io::Error),

    #[error("invalid version")]
    InvalidVersion,

    #[error("invalid format")]
    InvalidFormat,

    #[error("too many blocks for inodes")]
    TooManyInodeBlocks,
}
