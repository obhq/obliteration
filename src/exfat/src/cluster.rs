use crate::param::Params;
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::SeekFrom::Start;
use std::io::{Read, Seek};

pub(crate) struct ClusterReader<'a, I: Read + Seek> {
    image: &'a mut I,
    begin: u64,
    end: u64,
    offset: u64,
}

impl<'a, I: Read + Seek> ClusterReader<'a, I> {
    pub fn new(params: &Params, image: &'a mut I, index: usize) -> Result<Self, NewError> {
        if index < 2 {
            return Err(NewError::InvalidIndex);
        }

        let cluster = index as u64 - 2;
        let first_sector = params.cluster_heap_offset + (params.sectors_per_cluster * cluster);
        let begin = params.bytes_per_sector * first_sector;

        match image.seek(Start(begin)) {
            Ok(v) => {
                if v != begin {
                    return Err(NewError::InvalidClusterHeapOffset);
                }
            }
            Err(e) => return Err(NewError::IoFailed(e)),
        }

        Ok(Self {
            image,
            begin,
            end: params.bytes_per_sector * (first_sector + params.sectors_per_cluster),
            offset: 0,
        })
    }
}

impl<'a, I: Read + Seek> Read for ClusterReader<'a, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let offset = self.begin + self.offset;

        if buf.is_empty() || offset == self.end {
            return Ok(0);
        }

        let remain = self.end - offset;
        let amount = min(buf.len(), remain as usize);
        let read = self.image.read(&mut buf[..amount])?;

        self.offset += read as u64;

        Ok(read)
    }
}

#[derive(Debug)]
pub enum NewError {
    InvalidIndex,
    InvalidClusterHeapOffset,
    IoFailed(std::io::Error),
}

impl Error for NewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for NewError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIndex => f.write_str("the specified index is not valid"),
            Self::InvalidClusterHeapOffset => f.write_str("invalid ClusterHeapOffset"),
            Self::IoFailed(_) => f.write_str("I/O failed"),
        }
    }
}
