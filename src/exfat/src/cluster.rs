use crate::fat::Fat;
use crate::param::Params;
use std::cmp::min;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use thiserror::Error;

/// A cluster reader to read all data in a cluster chain.
pub(crate) struct ClustersReader<'a, I: Read + Seek> {
    params: &'a Params,
    image: &'a mut I,
    chain: Vec<usize>,
    cluster_size: u64, // In bytes.
    tail_size: u64,    // In bytes.
    cluster: usize,    // Index into chain.
    offset: u64,       // Offset into the current cluster.
}

impl<'a, I: Read + Seek> ClustersReader<'a, I> {
    pub fn new(
        params: &'a Params,
        fat: &Fat,
        image: &'a mut I,
        first_cluster: usize,
        data_length: Option<u64>,
        no_fat_chain: Option<bool>,
    ) -> Result<Self, NewError> {
        if first_cluster < 2 {
            return Err(NewError::InvalidFirstCluster);
        }

        // Get cluster chain.
        let cluster_size = params.cluster_size();
        let (chain, data_length) = if no_fat_chain.unwrap_or(false) {
            // If the NoFatChain bit is 1 then DataLength must not be zero.
            let data_length = match data_length {
                Some(v) if v > 0 => v,
                _ => return Err(NewError::InvalidDataLength),
            };

            // FIXME: Use div_ceil once https://github.com/rust-lang/rust/issues/88581 stabilized.
            let count = (data_length + cluster_size - 1) / cluster_size;
            let chain: Vec<usize> = (first_cluster..(first_cluster + count as usize)).collect();

            (chain, data_length)
        } else {
            let chain: Vec<usize> = fat.get_cluster_chain(first_cluster).collect();

            if chain.is_empty() {
                return Err(NewError::InvalidFirstCluster);
            }

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

            (chain, data_length)
        };

        // Seek to first cluster.
        let tail_size = data_length % cluster_size;

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

#[derive(Debug, Error)]
pub enum NewError {
    #[error("first cluster is not valid")]
    InvalidFirstCluster,

    #[error("data length is not valid")]
    InvalidDataLength,

    #[error("I/O failed")]
    IoFailed(#[source] std::io::Error),
}
