use std::path::PathBuf;
use thiserror::Error;

pub fn read_data_root() -> Result<Option<PathBuf>, DataRootError> {
    todo!()
}

/// Represents an error when read or write data root fails.
#[derive(Debug, Error)]
pub enum DataRootError {}
