use self::dirent::Dirent;
use crate::file::File;
use crate::inode::Inode;
use crate::Pfs;
use std::collections::HashMap;
use std::io::SeekFrom;
use std::ops::DerefMut;
use std::sync::Arc;
use thiserror::Error;
use util::mem::new_buffer;

pub mod dirent;

/// Represents a directory in the PFS.
#[derive(Clone)]
pub struct Directory<'a> {
    pfs: Arc<Pfs<'a>>,
    inode: usize,
}

impl<'a> Directory<'a> {
    pub(super) fn new(pfs: Arc<Pfs<'a>>, inode: usize) -> Self {
        Self { pfs, inode }
    }

    pub fn mode(&self) -> u16 {
        self.inode().mode()
    }

    pub fn flags(&self) -> u32 {
        self.inode().flags().value()
    }

    pub fn atime(&self) -> u64 {
        self.inode().atime()
    }

    pub fn mtime(&self) -> u64 {
        self.inode().mtime()
    }

    pub fn ctime(&self) -> u64 {
        self.inode().ctime()
    }

    pub fn birthtime(&self) -> u64 {
        self.inode().birthtime()
    }

    pub fn mtimensec(&self) -> u32 {
        self.inode().mtimensec()
    }

    pub fn atimensec(&self) -> u32 {
        self.inode().atimensec()
    }

    pub fn ctimensec(&self) -> u32 {
        self.inode().ctimensec()
    }

    pub fn birthnsec(&self) -> u32 {
        self.inode().birthnsec()
    }

    pub fn uid(&self) -> u32 {
        self.inode().uid()
    }

    pub fn gid(&self) -> u32 {
        self.inode().gid()
    }

    pub fn open(&self) -> Result<Items<'a>, OpenError> {
        // Load occupied blocks.
        let mut image = self.pfs.image.lock().unwrap();
        let image = image.deref_mut();
        let inode = &self.pfs.inodes[self.inode];
        let blocks = match inode.load_blocks(image.as_mut()) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::LoadBlocksFailed(e)),
        };

        // Read all dirents.
        let mut items: HashMap<Vec<u8>, Item<'a>> = HashMap::new();
        let block_size = image.header().block_size();
        let mut block_data = unsafe { new_buffer(block_size as usize) };

        for block_num in blocks {
            // Seek to block.
            let offset = (block_num as u64) * (block_size as u64);

            match image.seek(SeekFrom::Start(offset)) {
                Ok(v) => {
                    if v != offset {
                        return Err(OpenError::BlockNotExists(block_num));
                    }
                }
                Err(e) => return Err(OpenError::SeekToBlockFailed(block_num, e)),
            }

            // Read block data.
            if let Err(e) = image.read_exact(&mut block_data) {
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

                // Check if inode valid.
                let inode = dirent.inode();

                if inode >= self.pfs.inodes.len() {
                    return Err(OpenError::InvalidInode(inode));
                }

                // Construct object.
                let item = match dirent.ty() {
                    Dirent::FILE => Item::File(File::new(self.pfs.clone(), inode)),
                    Dirent::DIRECTORY => Item::Directory(Directory::new(self.pfs.clone(), inode)),
                    Dirent::SELF | Dirent::PARENT => continue,
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

        Ok(Items { items })
    }

    fn inode(&self) -> &Inode {
        &self.pfs.inodes[self.inode]
    }
}

/// Represents a collection of items in the directory.
pub struct Items<'a> {
    items: HashMap<Vec<u8>, Item<'a>>,
}

impl<'a> Items<'a> {
    pub fn get(&self, name: &[u8]) -> Option<&Item<'a>> {
        self.items.get(name)
    }

    pub fn take(&mut self, name: &[u8]) -> Option<Item<'a>> {
        self.items.remove(name)
    }
}

impl<'a> IntoIterator for Items<'a> {
    type Item = (Vec<u8>, Item<'a>);
    type IntoIter = std::collections::hash_map::IntoIter<Vec<u8>, Item<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Represents an item in the directory.
pub enum Item<'a> {
    Directory(Directory<'a>),
    File(File<'a>),
}

/// Errors of [`open()`][Directory::open()].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("inode #{0} is not valid")]
    InvalidInode(usize),

    #[error("cannot load occupied blocks")]
    LoadBlocksFailed(#[source] crate::inode::LoadBlocksError),

    #[error("cannot seek to block #{0}")]
    SeekToBlockFailed(u32, #[source] std::io::Error),

    #[error("block #{0} does not exist")]
    BlockNotExists(u32),

    #[error("cannot read block #{0}")]
    ReadBlockFailed(u32, #[source] std::io::Error),

    #[error("Dirent #{dirent:} in block #{block:} has invalid size")]
    InvalidDirent { block: u32, dirent: usize },

    #[error("Dirent #{dirent:} in block #{block:} has unknown type")]
    UnknownDirent { block: u32, dirent: usize },
}
