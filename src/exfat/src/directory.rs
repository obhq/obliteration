use crate::cluster::ClusterReader;
use crate::fat::Fat;
use crate::param::Params;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};
use util::mem::read_u8;

pub struct Directory {}

impl Directory {
    pub(super) fn open<I: Read + Seek>(
        params: &Params,
        fat: &Fat,
        image: &mut I,
        first_cluster: usize,
    ) -> Result<Self, OpenError> {
        // Enumerate cluster chain.
        'cluster_chain: for cluster_index in fat.get_cluster_chain(first_cluster) {
            // Create cluster reader.
            let mut cluster = match ClusterReader::new(params, image, cluster_index) {
                Ok(v) => v,
                Err(e) => return Err(OpenError::CreateClusterReaderFailed(cluster_index, e)),
            };

            // Enumerate directory entries.
            for entry_index in 0usize.. {
                // Read primary directory entry.
                let entry: [u8; 32] = match util::io::read_array(&mut cluster) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(OpenError::ReadEntryFailed {
                            entry: entry_index,
                            cluster: cluster_index,
                            error: e,
                        });
                    }
                };

                let entry = entry.as_ptr();
                let ty = EntryType(read_u8(entry, 0));

                // Check if end of entry.
                if !ty.is_regular() {
                    break 'cluster_chain;
                }
            }
        }

        Ok(Self {})
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct EntryType(u8);

impl EntryType {
    pub fn is_regular(self) -> bool {
        self.0 >= 0x81
    }

    pub fn type_code(self) -> u8 {
        self.0 & 0x1f
    }

    pub fn type_importance(self) -> u8 {
        (self.0 & !0x20) >> 5
    }

    pub fn type_category(self) -> u8 {
        (self.0 & !0x40) >> 6
    }
}

#[derive(Debug)]
pub enum OpenError {
    CreateClusterReaderFailed(usize, crate::cluster::NewError),
    ReadEntryFailed {
        entry: usize,
        cluster: usize,
        error: std::io::Error,
    },
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateClusterReaderFailed(_, e) => Some(e),
            Self::ReadEntryFailed {
                entry: _,
                cluster: _,
                error,
            } => Some(error),
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateClusterReaderFailed(i, _) => {
                write!(f, "cannot create reader for cluster #{}", i)
            }
            Self::ReadEntryFailed {
                entry,
                cluster,
                error: _,
            } => write!(
                f,
                "cannot read directory entry #{} from cluster #{}",
                entry, cluster
            ),
        }
    }
}
