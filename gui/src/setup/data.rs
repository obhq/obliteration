use super::SetupError;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
pub fn read_data_root() -> Result<Option<PathBuf>, SetupError> {
    todo!()
}

#[cfg(target_os = "macos")]
pub fn read_data_root() -> Result<Option<PathBuf>, SetupError> {
    todo!()
}

#[cfg(target_os = "windows")]
pub fn read_data_root() -> Result<Option<PathBuf>, SetupError> {
    todo!()
}
