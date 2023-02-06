use crate::cluster::ClustersReader;
use crate::entries::{ClusterAllocation, EntriesReader, EntryType, FileEntry, StreamEntry};
use crate::file::File;
use crate::image::Image;
use std::io::{Read, Seek};
use std::ops::DerefMut;
use std::sync::Arc;
use thiserror::Error;

/// Represents a directory in the exFAT.
pub struct Directory<I: Read + Seek> {
    image: Arc<Image<I>>,
    name: String,
    stream: StreamEntry,
}

impl<I: Read + Seek> Directory<I> {
    pub(crate) fn new(image: Arc<Image<I>>, name: String, stream: StreamEntry) -> Self {
        Self {
            image,
            name,
            stream,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn open(&self) -> Result<Vec<Item<I>>, OpenError> {
        // Create an entries reader.
        let params = self.image.params();
        let fat = self.image.fat();
        let mut image = self.image.reader();
        let alloc = self.stream.allocation();
        let no_fat_chain = Some(self.stream.no_fat_chain());
        let mut reader =
            match ClustersReader::from_alloc(params, fat, image.deref_mut(), alloc, no_fat_chain) {
                Ok(v) => EntriesReader::new(v),
                Err(e) => return Err(OpenError::CreateClustersReaderFailed(alloc.clone(), e)),
            };

        // Read file entries.
        let mut items: Vec<Item<I>> = Vec::new();

        loop {
            // Read primary entry.
            let entry = match reader.read() {
                Ok(v) => v,
                Err(e) => return Err(OpenError::ReadEntryFailed(e)),
            };

            // Check entry type.
            let ty = entry.ty();

            if !ty.is_regular() {
                break;
            } else if ty.type_category() != EntryType::PRIMARY {
                return Err(OpenError::NotPrimaryEntry(entry.index(), entry.cluster()));
            } else if ty.type_importance() != EntryType::CRITICAL || ty.type_code() != 5 {
                return Err(OpenError::NotFileEntry(entry.index(), entry.cluster()));
            }

            // Parse file entry.
            let file = match FileEntry::load(entry, &mut reader) {
                Ok(v) => v,
                Err(e) => return Err(OpenError::LoadFileEntryFailed(e)),
            };

            // Construct item.
            let name = file.name;
            let attrs = file.attributes;
            let stream = file.stream;

            let item = if attrs.is_directory() {
                Item::Directory(Directory::new(self.image.clone(), name, stream))
            } else {
                Item::File(File::new(self.image.clone(), name, stream))
            };

            items.push(item);
        }

        Ok(items)
    }
}

/// Represents an item in the directory.
pub enum Item<I: Read + Seek> {
    Directory(Directory<I>),
    File(File<I>),
}

/// Represents an error for [`open()`][Directory::open].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot create a clusters reader for allocation {0}")]
    CreateClustersReaderFailed(ClusterAllocation, #[source] crate::cluster::FromAllocError),

    #[error("cannot read an entry")]
    ReadEntryFailed(#[source] crate::entries::ReaderError),

    #[error("entry #{0} on cluster #{1} is not a primary entry")]
    NotPrimaryEntry(usize, usize),

    #[error("entry #{0} on cluster #{1} is not a file entry")]
    NotFileEntry(usize, usize),

    #[error("cannot load file entry")]
    LoadFileEntryFailed(#[source] crate::entries::FileEntryError),
}
