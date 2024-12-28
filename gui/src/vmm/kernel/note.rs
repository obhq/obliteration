use bytes::{Buf, Bytes};
use std::cmp::min;
use thiserror::Error;

/// Iterator to enumerate ELF notes.
pub struct Notes(Bytes);

impl Notes {
    pub(super) fn new(data: impl Into<Bytes>) -> Self {
        Self(data.into())
    }
}

impl Iterator for Notes {
    type Item = Result<Note, NoteError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check remaining data.
        let hdr = 4 * 3;

        if self.0.is_empty() {
            return None;
        } else if self.0.len() < hdr {
            return Some(Err(NoteError::InvalidHeader));
        }

        // Parse header.
        let mut hdr = self.0.split_to(hdr);
        let nlen: usize = hdr.get_u32_ne().try_into().unwrap();
        let dlen: usize = hdr.get_u32_ne().try_into().unwrap();
        let ty = hdr.get_u32_ne();

        if nlen == 0 {
            // Name is null-terminated so it should have at least 1 byte.
            return Some(Err(NoteError::InvalidName));
        } else if nlen > self.0.len() {
            return Some(Err(NoteError::InvalidHeader));
        }

        // Get name.
        let mut name = self.0.split_to(nlen);
        let len = nlen - 1;

        if name.iter().position(|&b| b == 0) != Some(len) {
            return Some(Err(NoteError::InvalidName));
        }

        name.truncate(len);

        // Skip alignment.
        let skip = nlen.next_multiple_of(4) - nlen;

        self.0.advance(min(skip, self.0.len()));

        if dlen > self.0.len() {
            return Some(Err(NoteError::InvalidHeader));
        }

        // Get description.
        let desc = self.0.split_to(dlen);
        let skip = dlen.next_multiple_of(4) - dlen;

        self.0.advance(min(skip, self.0.len()));

        Some(Ok(Note { name, desc, ty }))
    }
}

/// Parsed ELF program header.
pub struct Note {
    pub name: Bytes,
    pub desc: Bytes,
    pub ty: u32,
}

/// Represents an error when [`Notes`] fails to enumerate next ELF note.
#[derive(Debug, Error)]
pub enum NoteError {
    #[error("invalid header")]
    InvalidHeader,

    #[error("invalid name")]
    InvalidName,
}
