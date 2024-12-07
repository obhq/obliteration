use thiserror::Error;

pub fn read_data_root() -> Result<Option<String>, DataRootError> {
    todo!()
}

pub fn write_data_root(path: impl AsRef<str>) -> Result<(), DataRootError> {
    todo!()
}

/// Represents an error when read or write data root fails.
#[derive(Debug, Error)]
pub enum DataRootError {}
