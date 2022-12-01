use crate::fat::Fat;
use crate::param::Params;
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{ErrorKind, Read, Seek, SeekFrom};

pub(crate) struct ClustersReader<'a, I: Read + Seek> {
    params: &'a Params,
    image: &'a mut I,
    chain: Vec<usize>,
    cluster_size: u64, // in bytes
    tail_size: u64,
    cluster: usize, // index into chain
    offset: u64,    // offset into current cluster
}

impl<'a, I: Read + Seek> ClustersReader<'a, I> {
    pub fn new(
        params: &'a Params,
        fat: &Fat,
        image: &'a mut I,
        first_cluster: usize,
        data_length: Option<u64>,
    ) -> Result<Self, NewError> {
        if first_cluster < 2 {
            return Err(NewError::InvalidFirstCluster);
        }

        // Get cluster chain.
        let chain: Vec<usize> = fat.get_cluster_chain(first_cluster).collect();

        if chain.is_empty() {
            return Err(NewError::InvalidFirstCluster);
        }

        // Get data length.
        let cluster_size = params.bytes_per_sector * params.sectors_per_cluster;
        let data_length = match data_length {
            Some(v) => {
                if v > cluster_size * chain.len() as u64 {
                    return Err(NewError::InvalidDataLength);
                } else {
                    v
                }
            }
            None => params.bytes_per_sector * (params.sectors_per_cluster * chain.len() as u64),
        };

        let tail_size = data_length % cluster_size;

        // Seek to first cluster.
        let mut reader = Self {
            params,
            image,
            chain,
            cluster_size,
            tail_size: if tail_size == 0 {
                cluster_size
            } else {
                tail_size
            },
            cluster: 0,
            offset: 0,
        };

        if let Err(e) = reader.seek() {
            return Err(NewError::IoFailed(e));
        }

        Ok(reader)
    }

    pub fn cluster(&self) -> usize {
        self.chain[self.cluster]
    }

    fn seek(&mut self) -> Result<(), std::io::Error> {
        // Get offset into image.
        let cluster = self.cluster();
        let offset = match self.params.cluster_offset(cluster) {
            Some(v) => v + self.offset,
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    format!("cluster #{} is not available", cluster),
                ));
            }
        };

        // Seek image reader.
        match self.image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(std::io::Error::new(
                        ErrorKind::Other,
                        format!("cluster #{} is not available", cluster),
                    ));
                }
            }
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

impl<'a, I: Read + Seek> Read for ClustersReader<'a, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Check if the actual read is required.
        if buf.is_empty() || self.cluster == self.chain.len() {
            return Ok(0);
        }

        // Get cluster size.
        let cluster_size = if self.cluster == self.chain.len() - 1 {
            self.tail_size
        } else {
            self.cluster_size
        };

        // Read image.
        let remaining = cluster_size - self.offset;
        let target = min(buf.len(), remaining as usize);
        let read = self.image.read(&mut buf[..target])?;

        if read == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        self.offset += read as u64;

        // Check if all data in the current cluster is read.
        if self.offset == cluster_size {
            self.cluster += 1;
            self.offset = 0;

            if self.cluster < self.chain.len() {
                self.seek()?;
            }
        }

        Ok(read)
    }
}

#[derive(Debug)]
pub enum NewError {
    InvalidFirstCluster,
    InvalidDataLength,
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
            Self::InvalidFirstCluster => f.write_str("first cluster is not valid"),
            Self::InvalidDataLength => f.write_str("data length is not valid"),
            Self::IoFailed(_) => f.write_str("I/O failed"),
        }
    }
}
