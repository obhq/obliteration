use self::directory::Directory;
use self::inode::{BlockPointers, Inode};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::{new_buffer, uninit};

pub mod directory;
pub mod header;
pub mod inode;
pub mod reader;

pub struct Pfs<'image> {
    reader: Box<dyn reader::Reader + 'image>,
    header: header::Header,
    inodes: Vec<Inode>,
    super_root: Directory,
}

impl<'image> Pfs<'image> {
    pub fn open(
        image: &'image [u8],
        flags: ImageFlags,
        ekpfs: Option<&[u8]>,
    ) -> Result<Self, OpenError> {
        // Read header.
        let header = match header::Header::read(image) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::InvalidHeader(e)),
        };

        // Construct reader.
        let block_size = header.block_size();
        let reader: Box<dyn reader::Reader + 'image> = if header.mode().is_encrypted() {
            let ekpfs = match ekpfs {
                Some(v) => v,
                None => panic!("The image is encrypted but no EKPFS is provided"),
            };

            let key_seed = header.key_seed();
            let (data_key, tweak_key) = Self::derive_block_key(flags, ekpfs, key_seed);
            let reader = reader::EncryptedReader::new(image, data_key, tweak_key, block_size);

            Box::new(reader)
        } else {
            Box::new(reader::UnencryptedReader::new(image))
        };

        // Read inode blocks.
        let mut pfs = Self {
            inodes: Vec::with_capacity(header.inode_count()),
            reader,
            header,
            super_root: Directory::empty(),
        };

        for block_num in 0..pfs.header.inode_block_count() {
            let completed = match pfs.load_inodes(block_num) {
                Ok(v) => v,
                Err(e) => return Err(OpenError::ReadBlockFailed(block_num + 1, e)),
            };

            if completed {
                break;
            }
        }

        // Load super-root.
        pfs.super_root = match pfs.load_directory(pfs.header.super_root_inode()) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::LoadSuperRootFailed(e)),
        };

        Ok(pfs)
    }

    fn load_directory(&self, inode: usize) -> Result<Directory, LoadDirectoryError> {
        let blocks = match self.load_blocks(inode) {
            Ok(v) => v,
            Err(e) => return Err(LoadDirectoryError::LoadBlocksFailed(e)),
        };

        Ok(Directory::empty())
    }

    fn load_blocks(&self, inode_index: usize) -> Result<Vec<usize>, LoadBlocksError> {
        // Get target inode.
        let inode = match self.inodes.get(inode_index) {
            Some(v) => v,
            None => return Err(LoadBlocksError::InvalidInode),
        };

        // Check if inode use contiguous blocks.
        let mut pointers: Vec<usize> = Vec::with_capacity(inode.block_count());

        if pointers.len() == inode.block_count() {
            // This should not be possible but just in case for malformed image.
            return Ok(pointers);
        }

        if inode.direct_blocks()[1] == 0xffffffff {
            let start = inode.direct_blocks()[0] as usize;

            for block in start..(start + inode.block_count()) {
                pointers.push(block);
            }

            return Ok(pointers);
        }

        // Load direct pointers.
        for i in 0..12 {
            pointers.push(inode.direct_blocks()[i] as _);

            if pointers.len() == inode.block_count() {
                return Ok(pointers);
            }
        }

        // Load indirect 0.
        let block_num = inode.indirect_blocks()[0] as usize;
        let block_size = self.header.block_size();
        let mut block0 = new_buffer(block_size);

        if let Err(e) = self.reader.read(block_num * block_size, &mut block0) {
            return Err(LoadBlocksError::ReadBlockFailed(block_num, e));
        }

        for i in BlockPointers::new(self.header.mode(), &block0) {
            pointers.push(i);

            if pointers.len() == inode.block_count() {
                return Ok(pointers);
            }
        }

        // Load indirect 1.
        let block_num = inode.indirect_blocks()[1] as usize;

        if let Err(e) = self.reader.read(block_num * block_size, &mut block0) {
            return Err(LoadBlocksError::ReadBlockFailed(block_num, e));
        }

        let mut block1 = new_buffer(block_size);

        for i in BlockPointers::new(self.header.mode(), &block0) {
            if let Err(e) = self.reader.read(i * block_size, &mut block1) {
                return Err(LoadBlocksError::ReadBlockFailed(i, e));
            }

            for j in BlockPointers::new(self.header.mode(), &block1) {
                pointers.push(j);

                if pointers.len() == inode.block_count() {
                    return Ok(pointers);
                }
            }
        }

        panic!(
            "Data of inode #{} was spanned to indirect block #2, which we are not supported yet",
            inode_index
        );
    }

    /// `block_num` is a number of inode block, not image block.
    fn load_inodes(&mut self, block_num: usize) -> Result<bool, reader::ReadError> {
        // Get the offset for target block. The first inode block always start at image second
        // block.
        let block_size = self.header.block_size();
        let offset = block_size + block_num * block_size;

        // Read the whole block.
        let mut block_data = new_buffer(block_size);

        if let Err(e) = self.reader.read(offset, block_data.as_mut_slice()) {
            return Err(e);
        }

        // Read inodes in the block.
        let mut src = block_data.as_slice();

        if self.header.mode().is_64bits() {
            panic!("64-bits inode is not supported yet");
        }

        let reader = if self.header.mode().is_signed() {
            Inode::read_signed
        } else {
            Inode::read_unsigned
        };

        while self.inodes.len() < self.header.inode_count() {
            let inode = match reader(&mut src) {
                Ok(v) => v,
                Err(e) => match e {
                    inode::ReadError::TooSmall => return Ok(false),
                    inode::ReadError::IoFailed(e) => {
                        panic!("Failed to read inode from a buffer: {}", e);
                    }
                },
            };

            self.inodes.push(inode);
        }

        Ok(true)
    }

    /// Gets data key and tweak key.
    fn derive_block_key(flags: ImageFlags, ekpfs: &[u8], seed: &[u8; 16]) -> ([u8; 16], [u8; 16]) {
        // Derive EKPFS from seed if PFS use new encryption.
        let ekpfs: Vec<u8> = if flags.is_new_encryption() {
            let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs).unwrap();
            hmac.update(seed);
            hmac.finalize().into_bytes().to_vec()
        } else {
            ekpfs.into()
        };

        // Derive key.
        let mut hmac = Hmac::<Sha256>::new_from_slice(ekpfs.as_slice()).unwrap();
        let mut input: Vec<u8> = Vec::with_capacity(seed.len() + 4);

        input.extend(&[0x01, 0x00, 0x00, 0x00]);
        input.extend(seed);

        hmac.update(input.as_slice());

        // Split key.
        let secret = hmac.finalize().into_bytes();
        let mut data_key: [u8; 16] = uninit();
        let mut tweak_key: [u8; 16] = uninit();

        tweak_key.copy_from_slice(&secret[..16]);
        data_key.copy_from_slice(&secret[16..]);

        (data_key, tweak_key)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ImageFlags(u64);

impl ImageFlags {
    pub fn is_new_encryption(&self) -> bool {
        self.0 & 0x2000000000000000 != 0
    }
}

impl From<u64> for ImageFlags {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

#[derive(Debug)]
pub enum OpenError {
    InvalidHeader(header::ReadError),
    ReadBlockFailed(usize, reader::ReadError),
    LoadSuperRootFailed(LoadDirectoryError),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidHeader(e) => Some(e),
            Self::ReadBlockFailed(_, e) => Some(e),
            Self::LoadSuperRootFailed(e) => Some(e),
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(_) => f.write_str("invalid header"),
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
            Self::LoadSuperRootFailed(_) => f.write_str("cannot read super-root"),
        }
    }
}

#[derive(Debug)]
pub enum LoadDirectoryError {
    LoadBlocksFailed(LoadBlocksError),
}

impl Error for LoadDirectoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LoadBlocksFailed(e) => Some(e),
        }
    }
}

impl Display for LoadDirectoryError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::LoadBlocksFailed(_) => f.write_str("cannot read data blocks"),
        }
    }
}

#[derive(Debug)]
pub enum LoadBlocksError {
    InvalidInode,
    ReadBlockFailed(usize, reader::ReadError),
}

impl Error for LoadBlocksError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBlockFailed(_, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadBlocksError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidInode => f.write_str("invalid inode"),
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
        }
    }
}
