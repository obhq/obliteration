use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::Path;

// https://www.psdevwiki.com/ps4/Package_Files
pub struct PkgFile {
    raw: memmap2::Mmap,
}

impl PkgFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OpenError> {
        // Open file and map it to memory.
        let file = match File::open(path) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::OpenFailed(e)),
        };

        let raw = match unsafe { memmap2::Mmap::map(&file) } {
            Ok(v) => v,
            Err(e) => return Err(OpenError::MapFailed(e)),
        };

        Ok(Self { raw })
    }
}

#[derive(Debug)]
pub enum OpenError {
    OpenFailed(std::io::Error),
    MapFailed(std::io::Error),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OpenError::OpenFailed(e) => Some(e),
            OpenError::MapFailed(e) => Some(e),
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            OpenError::OpenFailed(e) => e.fmt(f),
            OpenError::MapFailed(e) => e.fmt(f),
        }
    }
}
