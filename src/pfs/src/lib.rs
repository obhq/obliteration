use self::directory::Directory;
use self::header::Header;
use self::inode::Inode;
use aes::cipher::KeyInit;
use aes::Aes128;
use generic_array::GenericArray;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use xts_mode::Xts128;

pub mod directory;
pub mod file;
pub mod header;
pub mod image;
pub mod inode;
pub mod pfsc;

pub fn open<'a, I>(mut image: I, ekpfs: Option<&[u8]>) -> Result<Directory<'a>, OpenError>
where
    I: Read + Seek + 'a,
{
    // Read header.
    let header = match Header::read(&mut image) {
        Ok(v) => v,
        Err(e) => return Err(OpenError::ReadHeaderFailed(e)),
    };

    // Check if image is supported.
    let mode = header.mode();

    if mode.is_64bits() {
        panic!("64-bits inode is not supported yet.");
    }

    // Construct reader.
    let block_size = header.block_size();
    let mut image: Box<dyn Image + 'a> = if mode.is_encrypted() {
        // The super block (block that contain header) never get encrypted.
        if (block_size as usize) < image::XTS_BLOCK_SIZE {
            return Err(OpenError::InvalidBlockSize);
        }

        // Get image size.
        let image_len = match image.seek(SeekFrom::End(0)) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::GetImageLengthFailed(e)),
        };

        // Setup decryptor.
        let ekpfs = match ekpfs {
            Some(v) => v,
            None => panic!("The image is encrypted but no EKPFS is provided"),
        };

        let key_seed = header.key_seed();
        let (data_key, tweak_key) = image::get_xts_keys(ekpfs, key_seed);
        let cipher_1 = Aes128::new(GenericArray::from_slice(&data_key));
        let cipher_2 = Aes128::new(GenericArray::from_slice(&tweak_key));

        Box::new(image::Encrypted::new(
            image,
            image_len,
            header,
            Xts128::<Aes128>::new(cipher_1, cipher_2),
            (block_size as usize) / image::XTS_BLOCK_SIZE,
            image_len,
        ))
    } else {
        Box::new(image::Unencrypted::new(image, header))
    };

    // Read inode blocks.
    let mut block_data = vec![0; block_size as usize];
    let mut inodes: Vec<Inode> = Vec::with_capacity(image.header().inode_count());

    'load_block: for block_num in 0..image.header().inode_block_count() {
        // Get the offset for target block. The first inode block always start at second block.
        let offset = (block_size as u64) + (block_num as u64) * (block_size as u64);

        // Seek to target block.
        match image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(OpenError::InvalidBlock(block_num));
                }
            }
            Err(e) => return Err(OpenError::SeekToBlockFailed(block_num, e)),
        }

        // Read the whole block.
        if let Err(e) = image.read_exact(&mut block_data) {
            return Err(OpenError::ReadBlockFailed(block_num, e));
        }

        // Read inodes in the block.
        let mut src = block_data.as_slice();

        let reader = if mode.is_signed() {
            Inode::from_raw32_signed
        } else {
            Inode::from_raw32_unsigned
        };

        while inodes.len() < image.header().inode_count() {
            let inode = match reader(inodes.len(), &mut src) {
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

    // Check if super-root valid
    let super_root = image.header().super_root_inode();

    if super_root >= inodes.len() {
        return Err(OpenError::InvalidSuperRoot);
    }

    // Construct super-root.
    let pfs = Pfs {
        image: Mutex::new(image),
        inodes,
    };

    Ok(Directory::new(Arc::new(pfs), super_root))
}

/// Represents a loaded PFS.
pub(crate) struct Pfs<'a> {
    image: Mutex<Box<dyn Image + 'a>>,
    inodes: Vec<Inode>,
}

/// Encapsulate a PFS image.
pub(crate) trait Image: Read + Seek {
    fn header(&self) -> &Header;
}

/// Errors for [`open()`].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot read header")]
    ReadHeaderFailed(#[source] header::ReadError),

    #[error("invalid block size")]
    InvalidBlockSize,

    #[error("cannot get the length of image")]
    GetImageLengthFailed(#[source] std::io::Error),

    #[error("cannot seek to block #{0}")]
    SeekToBlockFailed(u32, #[source] std::io::Error),

    #[error("block #{0} is not valid")]
    InvalidBlock(u32),

    #[error("cannot read block #{0}")]
    ReadBlockFailed(u32, #[source] std::io::Error),

    #[error("invalid super-root")]
    InvalidSuperRoot,
}
