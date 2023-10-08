use byteorder::{ByteOrder, BE, LE};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use thiserror::Error;

/// A loaded param.sfo.
///
/// See https://www.psdevwiki.com/ps4/Param.sfo#Internal_Structure for more information.
pub struct Param {
    title: String,
    title_id: String,
}

impl Param {
    pub fn read<R: Read + Seek>(mut raw: R) -> Result<Self, ReadError> {
        // Seek to the beginning.
        if let Err(e) = raw.seek(SeekFrom::Start(0)) {
            return Err(ReadError::SeekFailed(0, e));
        }

        // Read the header.
        let mut hdr = [0u8; 0x14];

        if let Err(e) = raw.read_exact(&mut hdr) {
            return Err(ReadError::ReadHeaderFailed(e));
        }

        // Check magic.
        let magic = BE::read_u32(&hdr[0x00..]);

        if magic != 0x00505346 {
            return Err(ReadError::InvalidMagic);
        }

        // Load the header.
        let key_table = LE::read_u32(&hdr[0x08..]) as u64;
        let data_table = LE::read_u32(&hdr[0x0C..]) as u64;
        let entries = LE::read_u32(&hdr[0x10..]) as u64;

        // Seek to key table.
        match raw.seek(SeekFrom::Start(key_table)) {
            Ok(v) => {
                if v != key_table {
                    return Err(ReadError::InvalidHeader);
                }
            }
            Err(e) => return Err(ReadError::SeekFailed(key_table, e)),
        }

        // Read key table.
        let mut keys = vec![0u8; 0xFFFF];
        let mut i = 0;

        while i != keys.len() {
            let r = match raw.read(&mut keys[i..]) {
                Ok(v) => v,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted {
                        continue;
                    }

                    return Err(ReadError::ReadKeyTableFailed(e));
                }
            };

            if r == 0 {
                break;
            }

            i += r;
        }

        keys.drain(i..);

        // Read entries.
        let mut title: Option<String> = None;
        let mut title_id: Option<String> = None;

        for i in 0..entries {
            // Seek to the entry.
            let offset = 0x14 + i * 0x10;

            match raw.seek(SeekFrom::Start(offset)) {
                Ok(v) => {
                    if v != offset {
                        return Err(ReadError::InvalidHeader);
                    }
                }
                Err(e) => return Err(ReadError::SeekFailed(offset, e)),
            }

            // Read the entry.
            let mut hdr = [0u8; 0x10];

            if let Err(e) = raw.read_exact(&mut hdr) {
                return Err(ReadError::ReadEntryFailed(i.try_into().unwrap(), e));
            }

            let key_offset = LE::read_u16(&hdr[0x00..]) as usize;
            let format = BE::read_u16(&hdr[0x02..]);
            let len = LE::read_u32(&hdr[0x04..]) as usize;
            let data_offset = data_table + LE::read_u32(&hdr[0x0C..]) as u64;

            if len == 0 {
                return Err(ReadError::InvalidEntry(i.try_into().unwrap()));
            }

            // Get key name.
            let key = match keys.get(key_offset..) {
                Some(v) => {
                    if let Some(i) = v.iter().position(|&b| b == 0) {
                        &v[..i]
                    } else {
                        return Err(ReadError::InvalidEntry(i.try_into().unwrap()));
                    }
                }
                None => return Err(ReadError::InvalidEntry(i.try_into().unwrap())),
            };

            // Seek to the value.
            match raw.seek(SeekFrom::Start(data_offset)) {
                Ok(v) => {
                    if v != data_offset {
                        return Err(ReadError::InvalidEntry(i.try_into().unwrap()));
                    }
                }
                Err(e) => return Err(ReadError::SeekFailed(data_offset, e)),
            }

            // Parse value.
            match key {
                b"TITLE" => title = Some(Self::read_utf8(&mut raw, i, format, len, 128)?),
                b"TITLE_ID" => title_id = Some(Self::read_utf8(&mut raw, i, format, len, 12)?),
                _ => continue,
            }
        }

        Ok(Self {
            title: title.ok_or(ReadError::MissingTitle)?,
            title_id: title_id.ok_or(ReadError::MissingTitleId)?,
        })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn title_id(&self) -> &str {
        &self.title_id
    }

    fn read_utf8<R: Read>(
        raw: &mut R,
        i: u64,
        format: u16,
        len: usize,
        max: usize,
    ) -> Result<String, ReadError> {
        // Check format and length.
        if format != 0x0402 || len > max {
            return Err(ReadError::InvalidEntry(i.try_into().unwrap()));
        }

        // Read the value.
        let mut data = vec![0u8; len];

        if let Err(e) = raw.read_exact(&mut data) {
            return Err(ReadError::ReadValueFailed(i.try_into().unwrap(), e));
        }

        // Check the value.
        if data.pop().unwrap() != 0 {
            return Err(ReadError::InvalidValue(i.try_into().unwrap()));
        }

        String::from_utf8(data).map_err(|_| ReadError::InvalidValue(i.try_into().unwrap()))
    }
}

/// Errors for reading param.sfo.
#[derive(Debug, Error)]
pub enum ReadError {
    #[error("cannot seek to {0:#018x}")]
    SeekFailed(u64, #[source] std::io::Error),

    #[error("cannot read the header")]
    ReadHeaderFailed(#[source] std::io::Error),

    #[error("invalid magic")]
    InvalidMagic,

    #[error("invalid header")]
    InvalidHeader,

    #[error("cannot read key table")]
    ReadKeyTableFailed(#[source] std::io::Error),

    #[error("cannot read entry #{0}")]
    ReadEntryFailed(usize, #[source] std::io::Error),

    #[error("entry #{0} is not valid")]
    InvalidEntry(usize),

    #[error("cannot read the value of entry #{0}")]
    ReadValueFailed(usize, #[source] std::io::Error),

    #[error("entry #{0} has invalid value")]
    InvalidValue(usize),

    #[error("TITLE is not found")]
    MissingTitle,

    #[error("TITLE_ID is not found")]
    MissingTitleId,
}
