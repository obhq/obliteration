use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Take};
use std::path::Path;
use thiserror::Error;

pub use self::segment::*;

mod segment;

/// Encapsulates a kernel ELF file.
pub struct Kernel {
    file: File,
    e_entry: usize,
    e_phoff: u64,
    e_phnum: u64,
}

impl Kernel {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, KernelError> {
        // Read ELF header.
        let mut file = File::open(path).map_err(KernelError::OpenImageFailed)?;
        let mut hdr = [0; 64];

        file.read_exact(&mut hdr)
            .map_err(KernelError::ReadElfHeaderFailed)?;

        // Check if ELF.
        if &hdr[..4] != b"\x7fELF" {
            return Err(KernelError::NotElf);
        }

        // Check ELF type.
        if hdr[4] != 2 {
            return Err(KernelError::Not64);
        }

        match hdr[6] {
            1 => {}
            v => return Err(KernelError::UnknownElfVersion(v)),
        }

        if u16::from_ne_bytes(hdr[18..20].try_into().unwrap()) != ELF_MACHINE {
            return Err(KernelError::DifferentArch);
        }

        // Load ELF header.
        let e_entry = usize::from_ne_bytes(hdr[24..32].try_into().unwrap());
        let e_phoff = u64::from_ne_bytes(hdr[32..40].try_into().unwrap());
        let e_phentsize: usize = u16::from_ne_bytes(hdr[54..56].try_into().unwrap()).into();
        let e_phnum: u64 = u16::from_ne_bytes(hdr[56..58].try_into().unwrap()).into();

        if e_phentsize != 56 {
            return Err(KernelError::UnsupportedProgramHeader);
        }

        Ok(Self {
            file,
            e_entry,
            e_phoff,
            e_phnum,
        })
    }

    pub fn entry(&self) -> usize {
        self.e_entry
    }

    pub fn program_headers(&mut self) -> Result<ProgramHeaders, std::io::Error> {
        let off = self.file.seek(SeekFrom::Start(self.e_phoff))?;

        if off != self.e_phoff {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
        } else {
            Ok(ProgramHeaders::new(&mut self.file, off, self.e_phnum))
        }
    }

    pub fn segment_data(&mut self, hdr: &ProgramHeader) -> Result<Take<&mut File>, std::io::Error> {
        let off = self.file.seek(SeekFrom::Start(hdr.p_offset))?;

        if off != hdr.p_offset {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
        } else {
            Ok(self.file.by_ref().take(hdr.p_filesz))
        }
    }
}

#[cfg(target_arch = "x86_64")]
const ELF_MACHINE: u16 = 62;
#[cfg(target_arch = "aarch64")]
const ELF_MACHINE: u16 = 183;

/// Represents an error when [`Kernel::open()`] fails.
#[derive(Debug, Error)]
pub enum KernelError {
    #[error("couldn't open kernel file")]
    OpenImageFailed(#[source] std::io::Error),

    #[error("couldn't read ELF header")]
    ReadElfHeaderFailed(#[source] std::io::Error),

    #[error("the kernel is not an ELF file")]
    NotElf,

    #[error("the kernel has unknown ELF version {0}")]
    UnknownElfVersion(u8),

    #[error("the kernel is not 64-bit kernel")]
    Not64,

    #[error("the kernel is for a different CPU architecture")]
    DifferentArch,

    #[error("the kernel has unsupported e_phentsize")]
    UnsupportedProgramHeader,
}
