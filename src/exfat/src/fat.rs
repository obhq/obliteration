use crate::param::Params;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use util::mem::new_buffer;
use util::slice::as_mut_bytes;

pub(super) struct Fat {
    entries: Vec<u32>,
}

impl Fat {
    pub fn load<I: Read + Seek>(
        params: &Params,
        image: &mut I,
        index: usize,
    ) -> Result<Self, LoadError> {
        // Seek to FAT region.
        let sector = match params.fat_length.checked_mul(index as u64) {
            Some(v) => match params.fat_offset.checked_add(v) {
                Some(v) => v,
                None => return Err(LoadError::InvalidFatOffset),
            },
            None => return Err(LoadError::InvalidFatLength),
        };

        let offset = match sector.checked_mul(params.bytes_per_sector) {
            Some(v) => v,
            None => return Err(LoadError::InvalidFatOffset),
        };

        match image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(LoadError::InvalidFatOffset);
                }
            }
            Err(e) => return Err(LoadError::IoFailed(e)),
        }

        // Load entries.
        let mut entries: Vec<u32> = unsafe{new_buffer(params.cluster_count + 2)};

        if let Err(e) = image.read_exact(as_mut_bytes(&mut entries)) {
            return Err(LoadError::IoFailed(e));
        }

        Ok(Self { entries })
    }

    pub fn get_cluster_chain(&self, first: usize) -> ClusterChain<'_> {
        ClusterChain {
            entries: &self.entries,
            next: first,
        }
    }
}

pub(crate) struct ClusterChain<'fat> {
    entries: &'fat [u32],
    next: usize,
}

impl<'fat> Iterator for ClusterChain<'fat> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // Check next entry.
        let entries = self.entries;
        let next = self.next;

        if next < 2 || next >= entries.len() || entries[next] == 0xfffffff7 {
            return None;
        }

        // Move to next entry.
        self.next = entries[next] as usize;

        Some(next)
    }
}

#[derive(Debug)]
pub enum LoadError {
    InvalidFatLength,
    InvalidFatOffset,
    IoFailed(std::io::Error),
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFatLength => f.write_str("invalid FatLength"),
            Self::InvalidFatOffset => f.write_str("invalid FatOffset"),
            Self::IoFailed(_) => f.write_str("I/O failed"),
        }
    }
}
