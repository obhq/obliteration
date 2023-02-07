use crate::cluster::ClustersReader;
use crate::entries::StreamEntry;
use crate::fat::Fat;
use crate::image::Image;
use crate::param::Params;
use std::io::{IoSliceMut, Read, Seek, SeekFrom};
use std::mem::transmute;
use std::ops::DerefMut;
use std::sync::{Arc, MutexGuard};
use thiserror::Error;

/// Represents a file in the exFAT.
pub struct File<I: Read + Seek> {
    image: Arc<Image<I>>,
    name: String,
    stream: StreamEntry,
}

impl<I: Read + Seek> File<I> {
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

    pub fn len(&self) -> u64 {
        self.stream.valid_data_length()
    }

    pub fn open(&mut self) -> Result<Option<FileReader<'_, I>>, OpenError> {
        // Check if file is empty.
        let alloc = self.stream.allocation();
        let first_cluster = alloc.first_cluster();

        if first_cluster == 0 {
            return Ok(None);
        }

        // Create a clusters reader.
        let params = self.image.params() as *const Params;
        let fat = self.image.fat() as *const Fat;
        let mut image = Box::new(self.image.reader());
        let reader = match ClustersReader::new(
            unsafe { transmute(params) },
            unsafe { transmute(fat) },
            unsafe { transmute(image.as_mut().deref_mut()) },
            first_cluster,
            Some(self.stream.valid_data_length()),
            Some(self.stream.no_fat_chain()),
        ) {
            Ok(v) => v,
            Err(e) => {
                return Err(OpenError::CreateClustersReaderFailed(
                    first_cluster,
                    self.stream.valid_data_length(),
                    e,
                ));
            }
        };

        Ok(Some(FileReader { image, reader }))
    }
}

/// A struct to read file data on exFAT.
pub struct FileReader<'a, I: Read + Seek> {
    reader: ClustersReader<'a, I>,

    // We need to keep this and drop it last because the reader is referencing this via a pointer.
    #[allow(unused)]
    image: Box<MutexGuard<'a, I>>,
}

impl<'a, I: Read + Seek> Seek for FileReader<'a, I> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.reader.rewind()
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.reader.stream_position()
    }
}

impl<'a, I: Read + Seek> Read for FileReader<'a, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.reader.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.reader.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.reader.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.reader.read_exact(buf)
    }
}

/// Represents an error for [`open()`][File::open()].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot create a clusters reader for allocation {0}:{1}")]
    CreateClustersReaderFailed(usize, u64, #[source] crate::cluster::NewError),
}
