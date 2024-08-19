use std::fs::File;
use std::io::Read;
use std::iter::FusedIterator;
use thiserror::Error;

/// Iterator to enumerate ELF program headers.
pub struct ProgramHeaders<'a> {
    file: &'a mut File,
    start: u64,
    count: u64,
    parsed: u64,
}

impl<'a> ProgramHeaders<'a> {
    pub(super) fn new(file: &'a mut File, start: u64, count: u64) -> Self {
        Self {
            file,
            start,
            count,
            parsed: 0,
        }
    }
}

impl<'a> Iterator for ProgramHeaders<'a> {
    type Item = Result<ProgramHeader, ProgramHeaderError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check remaining.
        if self.parsed == self.count {
            return None;
        }

        // Read data.
        let mut data = [0u8; 56];

        if let Err(e) = self.file.read_exact(&mut data) {
            return Some(Err(ProgramHeaderError::ReadFailed(
                self.start + self.parsed * 56,
                e,
            )));
        }

        // Parse data.
        let p_type = u32::from_ne_bytes(data[..4].try_into().unwrap());
        let p_offset = u64::from_ne_bytes(data[8..16].try_into().unwrap());
        let p_vaddr = usize::from_ne_bytes(data[16..24].try_into().unwrap());
        let p_filesz = u64::from_ne_bytes(data[32..40].try_into().unwrap());
        let p_memsz = usize::from_ne_bytes(data[40..48].try_into().unwrap());

        self.parsed += 1;

        Some(Ok(ProgramHeader {
            p_type,
            p_offset,
            p_vaddr,
            p_filesz,
            p_memsz,
        }))
    }
}

impl<'a> FusedIterator for ProgramHeaders<'a> {}

/// Parsed ELF program header.
pub struct ProgramHeader {
    pub p_type: u32,
    pub p_offset: u64,
    pub p_vaddr: usize,
    pub p_filesz: u64,
    pub p_memsz: usize,
}

/// Represents an error when [`ProgramHeaders`] fails to enumerate an ELF header.
#[derive(Debug, Error)]
pub enum ProgramHeaderError {
    #[error("couldn't read 56 bytes at offset {0}")]
    ReadFailed(u64, #[source] std::io::Error),
}
