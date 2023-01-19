use self::directory::Directory;
use self::header::Header;
use self::inode::Inode;
use aes::cipher::KeyInit;
use aes::Aes128;
use generic_array::GenericArray;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use util::mem::new_buffer;
use xts_mode::Xts128;

pub mod directory;
pub mod file;
pub mod header;
pub mod image;
pub mod inode;
pub mod pfsc;

pub fn open<'raw, R>(image: R, ekpfs: Option<&[u8]>) -> Result<Arc<dyn Image + 'raw>, OpenError>
where
    R: AsRef<[u8]> + 'raw,
{
    // Read header.
    let header = match Header::read(image.as_ref()) {
        Ok(v) => v,
        Err(e) => return Err(OpenError::InvalidHeader(e)),
    };

    // Construct reader.
    let block_size = header.block_size();
    let image: Arc<dyn Image + 'raw> = if header.mode().is_encrypted() {
        // The super block (block that contain header) never get encrypted.
        if (block_size as usize) < image::XTS_BLOCK_SIZE {
            return Err(OpenError::InvalidBlockSize);
        }

        // Setup decryptor.
        let ekpfs = match ekpfs {
            Some(v) => v,
            None => panic!("The image is encrypted but no EKPFS is provided"),
        };

        let key_seed = header.key_seed();
        let (data_key, tweak_key) = image::get_xts_keys(ekpfs, key_seed);
        let cipher_1 = Aes128::new(GenericArray::from_slice(&data_key));
        let cipher_2 = Aes128::new(GenericArray::from_slice(&tweak_key));

        Arc::new(image::Encrypted::new(
            image,
            header,
            Xts128::<Aes128>::new(cipher_1, cipher_2),
            (block_size as usize) / image::XTS_BLOCK_SIZE,
        ))
    } else {
        Arc::new(image::Unencrypted::new(image, header))
    };

    Ok(image)
}

pub fn mount<'image>(image: Arc<dyn Image + 'image>) -> Result<Pfs<'image>, MountError> {
    let header = image.header();

    if header.mode().is_64bits() {
        panic!("64-bits inode is not supported yet");
    }

    // Read inode blocks.
    let block_size = header.block_size();
    let mut block_data = new_buffer(block_size as usize);
    let mut inodes: Vec<Inode<'image>> = Vec::with_capacity(header.inode_count());

    'load_block: for block_num in 0..header.inode_block_count() {
        // Get the offset for target block. The first inode block always start at second block.
        let offset = (block_size as u64) + (block_num as u64) * (block_size as u64);

        // Read the whole block.
        if let Err(e) = image.read(offset as usize, &mut block_data) {
            return Err(MountError::ReadBlockFailed(block_num + 1, e));
        }

        // Read inodes in the block.
        let mut src = block_data.as_slice();

        let reader = if header.mode().is_signed() {
            Inode::from_raw32_signed
        } else {
            Inode::from_raw32_unsigned
        };

        while inodes.len() < header.inode_count() {
            let inode = match reader(image.clone(), inodes.len(), &mut src) {
                Ok(v) => v,
                Err(e) => match e {
                    inode::FromRawError::TooSmall => continue 'load_block,
                    inode::FromRawError::IoFailed(e) => {
                        panic!("Failed to read inode from a buffer: {}", e);
                    }
                },
            };

            inodes.push(inode);
        }

        break;
    }

    Ok(Pfs { image, inodes })
}

pub trait Image {
    fn header(&self) -> &Header;

    /// Fill `buf` from data beginning at `offset`.
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), ReadError>;
}

pub struct Pfs<'image> {
    image: Arc<dyn Image + 'image>,
    inodes: Vec<Inode<'image>>,
}

impl<'image> Pfs<'image> {
    pub fn open_super_root<'a>(&'a self) -> Result<Directory<'a, 'image>, OpenSuperRootError> {
        let header = self.image.header();
        let inode = match self.inodes.get(header.super_root_inode()) {
            Some(v) => v,
            None => return Err(OpenSuperRootError::InvalidInode),
        };

        Ok(Directory::new(self.image.clone(), &self.inodes, inode))
    }
}

#[derive(Debug)]
pub enum OpenError {
    InvalidHeader(header::ReadError),
    InvalidBlockSize,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidHeader(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(_) => f.write_str("invalid header"),
            Self::InvalidBlockSize => f.write_str("invalid block size"),
        }
    }
}

#[derive(Debug)]
pub enum MountError {
    ReadBlockFailed(u32, ReadError),
}

impl Error for MountError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBlockFailed(_, e) => Some(e),
        }
    }
}

impl Display for MountError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
        }
    }
}

#[derive(Debug)]
pub enum OpenSuperRootError {
    InvalidInode,
}

impl Error for OpenSuperRootError {}

impl Display for OpenSuperRootError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidInode => f.write_str("invalid inode"),
        }
    }
}

#[derive(Debug)]
pub enum ReadError {
    InvalidOffset,
}

impl Error for ReadError {}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidOffset => f.write_str("invalid offset"),
        }
    }
}
