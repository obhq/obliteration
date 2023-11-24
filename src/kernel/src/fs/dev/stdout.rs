use crate::errno::Errno;
use crate::fs::{VFile, VFileOps, VPath};
use crate::process::VThread;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};
use std::io::{self, Write};

/// An implementation of `/dev/stdout`.
#[derive(Debug)]
pub struct Stdout {}

impl Stdout {
    pub const PATH: &VPath = vpath!("/dev/stdout");

    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for Stdout {
    fn write(
        &self,
        _file: &VFile,
        data: &[u8],
        _cred: &Ucred,
        _td: &VThread,
    ) -> Result<usize, Box<dyn Errno>> {
        let stderr = io::stderr();
        let mut handle = stderr.lock();

        match handle.write(data) {
            Ok(ret) => ret,
            Err(e) => todo!("Encountered error {e} while writing to stderr."),
        }
    }

    fn ioctl(
        &self,
        _file: &VFile,
        _com: u64,
        _data: &[u8],
        _cred: &Ucred,
        _td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

impl Display for Stdout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}
