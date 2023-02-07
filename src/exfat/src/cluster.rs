use crate::fat::Fat;
use crate::param::Params;
use std::cmp::min;
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

/// A cluster reader to read all data in a cluster chain.
pub(crate) struct ClustersReader<'a, I: Read + Seek> {
    params: &'a Params,
    image: &'a mut I,
    chain: Vec<usize>,
    data_length: u64,
    offset: u64,
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
        let mut reader = Self {
            params,
            image,
            chain,
            data_length,
            offset: 0,
        };

        if let Err(e) = reader.seek_current_cluster() {
            return Err(NewError::SeekToFirstClusterFailed(e));
        }

        Ok(reader)
    }

    pub fn cluster(&self) -> usize {
        self.chain[(self.offset / self.params.cluster_size()) as usize]
    }

    fn seek_current_cluster(&mut self) -> Result<(), std::io::Error> {
        use std::io::{Error, ErrorKind};

        // Check if the offset is exactly at the cluster beginning.
        let cluster_size = self.params.cluster_size();

        if self.offset % cluster_size != 0 {
            panic!("The current offset must be at the beginning of the cluster.");
        }

        // Calculate an offset for the cluster.
        let cluster = self.chain[(self.offset / cluster_size) as usize];
        let offset = match self.params.cluster_offset(cluster) {
            Some(v) => v,
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("cluster #{cluster} does not exists in the image"),
                ));
            }
        };

        // Seek image to the cluster.
        match self.image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("cluster #{cluster} does not exists in the image"),
                    ));
                }
            }
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

impl<'a, I: Read + Seek> Seek for ClustersReader<'a, I> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        use std::io::{Error, ErrorKind};

        // Calculate target offset.
        let offset = match pos {
            SeekFrom::Start(v) => min(v, self.data_length),
            SeekFrom::End(v) => {
                if v >= 0 {
                    self.data_length
                } else if let Some(v) = self.data_length.checked_sub(v.unsigned_abs()) {
                    v
                } else {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    min(self.offset + (v as u64), self.data_length)
                } else if let Some(v) = self.offset.checked_sub(v.unsigned_abs()) {
                    v
                } else {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
            }
        };

        // Check if we need to do the actual seek.
        if offset != self.offset {
            // Calculate the offset for the cluster where the target offset is belong.
            let cluster_size = self.params.cluster_size();
            let cluster = self.chain[(offset / cluster_size) as usize];
            let cluster_offset = match self.params.cluster_offset(cluster) {
                Some(v) => v,
                None => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("cluster #{cluster} is not available"),
                    ));
                }
            };

            // Seek image to the target offset inside the cluster.
            let image_offset = cluster_offset + offset % cluster_size;

            match self.image.seek(SeekFrom::Start(image_offset)) {
                Ok(v) => {
                    if v != image_offset {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("offset {v} does not exists in the image"),
                        ));
                    }
                }
                Err(e) => return Err(e),
            }

            self.offset = offset;
        }

        Ok(offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        if self.offset != 0 {
            // Seek image to the first cluster.
            // We don't need to check if the first cluster is valid because we already checked it
            // inside new.
            let first_cluster = self.params.cluster_offset(self.chain[0]).unwrap();

            self.image.seek(SeekFrom::Start(first_cluster))?;

            // Set the offset.
            self.offset = 0;
        }

        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.offset)
    }
}

impl<'a, I: Read + Seek> Read for ClustersReader<'a, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Check if the actual read is required.
        if buf.is_empty() || self.offset == self.data_length {
            return Ok(0);
        }

        // Get remaining data in the current cluster.
        let cluster_size = self.params.cluster_size();
        let cluster_remaining = cluster_size - self.offset % cluster_size;
        let remaining = min(cluster_remaining, self.data_length - self.offset);

        // Read image.
        let amount = min(buf.len(), remaining as usize);

        self.image.read_exact(&mut buf[..amount])?;
        self.offset += amount as u64;

        // Check if we need to move to next cluster.
        if self.offset != self.data_length && amount == (cluster_remaining as usize) {
            if let Err(e) = self.seek_current_cluster() {
                // Reset offset back to the previous position.
                let previous_offset = self.offset - (amount as u64);

                if let Err(e) = self.seek(SeekFrom::Start(previous_offset)) {
                    panic!("Cannot seek back to the previous offset: {e}.");
                }

                // Don't do this before we invoke seek on the above.
                self.offset -= amount as u64;

                return Err(e);
            }
        }

        Ok(amount)
    }
}

/// Represents an error for [`new()`][ClustersReader::new()].
#[derive(Debug, Error)]
pub enum NewError {
    #[error("first cluster is not valid")]
    InvalidFirstCluster,

    #[error("data length is not valid")]
    InvalidDataLength,

    #[error("cannot seek to the first cluster")]
    SeekToFirstClusterFailed(#[source] std::io::Error),
}
