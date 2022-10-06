use self::dirent::Dirent;
use crate::file::File;
use crate::inode::Inode;
use crate::Image;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::new_buffer;

pub mod dirent;

pub struct Directory<'pfs, 'image, 'raw_image> {
    image: &'image (dyn Image + 'raw_image),
    inodes: &'pfs [Inode<'image, 'raw_image>],
    inode: &'pfs Inode<'image, 'raw_image>,
}

impl<'pfs, 'image, 'raw_image> Directory<'pfs, 'image, 'raw_image> {
    pub(super) fn new(
        image: &'image (dyn Image + 'raw_image),
        inodes: &'pfs [Inode<'image, 'raw_image>],
        inode: &'pfs Inode<'image, 'raw_image>,
    ) -> Self {
        Self {
            image,
            inodes,
            inode,
        }
    }

    pub fn open(&self) -> Result<Items<'pfs, 'image, 'raw_image>, OpenError> {
        // Load occupied blocks.
        let blocks = match self.inode.load_blocks() {
            Ok(v) => v,
            Err(e) => return Err(OpenError::LoadBlocksFailed(e)),
        };

        // Read all dirents.
        let mut parent: Option<Directory<'pfs, 'image, 'raw_image>> = None;
        let mut items: HashMap<Vec<u8>, Item<'pfs, 'image, 'raw_image>> = HashMap::new();
        let block_size = self.image.header().block_size();
        let mut block_data = new_buffer(block_size as usize);

        for block_num in blocks {
            // Read block data.
            let offset = (block_num as u64) * (block_size as u64);

            if let Err(e) = self.image.read(offset as usize, &mut block_data) {
                return Err(OpenError::ReadBlockFailed(block_num, e));
            }

            // Read dirents in the block.
            let mut next = block_data.as_slice();

            for num in 0.. {
                // Read dirent.
                let mut dirent = match Dirent::read(&mut next) {
                    Ok(v) => v,
                    Err(e) => match e {
                        dirent::ReadError::IoFailed(e) => {
                            panic!("Failed to read dirent due to I/O error: {}", e);
                        }
                        dirent::ReadError::TooSmall | dirent::ReadError::EndOfEntry => break,
                    },
                };

                let inode = match self.inodes.get(dirent.inode()) {
                    Some(v) => v,
                    None => return Err(OpenError::InvalidInode(dirent.inode())),
                };

                // Skip remaining padding.
                next = match next.get(dirent.padding_size()..) {
                    Some(v) => v,
                    None => {
                        return Err(OpenError::InvalidDirent {
                            block: block_num,
                            dirent: num,
                        });
                    }
                };

                // Construct object.
                if dirent.ty() == Dirent::PARENT {
                    parent = Some(Directory::new(self.image, self.inodes, inode));
                } else {
                    let item = match dirent.ty() {
                        Dirent::FILE => Item::File(File::new(self.image, inode)),
                        Dirent::DIRECTORY => {
                            Item::Directory(Directory::new(self.image, self.inodes, inode))
                        }
                        Dirent::SELF => continue,
                        _ => {
                            return Err(OpenError::UnknownDirent {
                                block: block_num,
                                dirent: num,
                            });
                        }
                    };

                    items.insert(dirent.take_name(), item);
                }
            }
        }

        Ok(Items { parent, items })
    }
}

pub struct Items<'pfs, 'image, 'raw_image> {
    parent: Option<Directory<'pfs, 'image, 'raw_image>>,
    items: HashMap<Vec<u8>, Item<'pfs, 'image, 'raw_image>>,
}

impl<'pfs, 'image, 'raw_image> IntoIterator for Items<'pfs, 'image, 'raw_image> {
    type Item = (Vec<u8>, Item<'pfs, 'image, 'raw_image>);
    type IntoIter = std::collections::hash_map::IntoIter<Vec<u8>, Item<'pfs, 'image, 'raw_image>>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

pub enum Item<'pfs, 'image, 'raw_image> {
    Directory(Directory<'pfs, 'image, 'raw_image>),
    File(File<'pfs, 'image, 'raw_image>),
}

#[derive(Debug)]
pub enum OpenError {
    InvalidInode(usize),
    LoadBlocksFailed(crate::inode::LoadBlocksError),
    ReadBlockFailed(u32, crate::ReadError),
    InvalidDirent { block: u32, dirent: usize },
    UnknownDirent { block: u32, dirent: usize },
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LoadBlocksFailed(e) => Some(e),
            Self::ReadBlockFailed(_, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidInode(i) => write!(f, "inode #{} is not valid", i),
            Self::LoadBlocksFailed(_) => f.write_str("cannot load occupied blocks"),
            Self::ReadBlockFailed(b, _) => write!(f, "cannot read block #{}", b),
            Self::InvalidDirent { block, dirent } => {
                write!(f, "Dirent #{} in block #{} has invalid size", dirent, block)
            }
            Self::UnknownDirent { block, dirent } => {
                write!(f, "Dirent #{} in block #{} has unknown type", dirent, block)
            }
        }
    }
}
