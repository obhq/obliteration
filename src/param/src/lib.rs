use byteorder::{ByteOrder, BE, LE};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use thiserror::Error;

/// A loaded param.sfo.
///
/// See https://www.psdevwiki.com/ps4/Param.sfo#Internal_Structure for more information.
pub struct Param {
    app_ver: String,
    category: String,
    content_id: String,
    title: String,
    title_id: String,
    version: String,
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
        let mut app_ver: Option<String> = None;
        let mut category: Option<String> = None;
        let mut content_id: Option<String> = None;
        let mut title: Option<String> = None;
        let mut title_id: Option<String> = None;
        let mut version: Option<String> = None;

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
                b"APP_VER" => {
                    app_ver = Some(Self::read_utf8(&mut raw, i, format, len, 8)?);
                }
                b"CATEGORY" => {
                    let category_param = Self::read_utf8(&mut raw, i, format, 4, 4)?;
                    category = Some(category_param.clone());
                    if !category_param.contains("bd") && !category_param.contains("ac")  // Blu-Ray Game, DLC.
                        // Check if this is a patch
                        && !category_param.starts_with("gp")
                        // Check if this is an application
                        && !category_param.starts_with("gd")
                    {
                        // For types such as gc, sd, la, and wda, there is no Title or TitleID.
                        title = Some(format!("No Title {}", category_param).to_string());
                        title_id = Some("No TitleID".to_string());
                    }
                }
                b"CONTENT_ID" => {
                    content_id = Some(Self::read_utf8(&mut raw, i, format, len, 48)?);
                }
                b"TITLE" => {
                    if title.is_none() {
                        title = Some(Self::read_utf8(&mut raw, i, format, len, 128)?);
                    }
                }
                b"TITLE_ID" => {
                    if title_id.is_none() {
                        title_id = Some(Self::read_utf8(&mut raw, i, format, len, 12)?);
                    }
                }
                b"VERSION" => {
                    version = Some(Self::read_utf8(&mut raw, i, format, len, 8)?);
                }
                _ => continue,
            }
        }

        Ok(Self {
            // App_Ver for Games and Patches, for DLC, use version. Anything else is abnormal.
            app_ver: app_ver
                .or(version.clone())
                .ok_or(ReadError::MissingVersion)?,
            category: category.ok_or(ReadError::MissingCategory)?,
            content_id: content_id.ok_or(ReadError::MissingContentId)?,
            title: title.ok_or(ReadError::MissingTitle)?,
            title_id: title_id.ok_or(ReadError::MissingTitleId)?,
            version: version.ok_or(ReadError::MissingVersion)?,
        })
    }

    /// Fetches the value APP_VER from given Param.SFO
    pub fn app_ver(&self) -> &str {
        &self.app_ver
    }

    /// Fetches the value CATEGORY from given Param.SFO
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Fetches the value CONTENT_ID from given Param.SFO
    pub fn content_id(&self) -> &str {
        &self.content_id
    }

    /// Fetches a shortened variant of value CONTENT_ID from given Param.SFO
    pub fn shortcontent_id(&self) -> &str {
        self.content_id
            .split('-')
            .last()
            .unwrap_or(&self.content_id)
    }

    /// Fetches the value TITLE from given Param.SFO
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Fetches the value TITLE_ID from given Param.SFO
    pub fn title_id(&self) -> &str {
        &self.title_id
    }

    /// Fetches the value VERSION from given Param.SFO
    pub fn version(&self) -> &str {
        &self.version
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

    #[error("APP_VER parameter not found")]
    MissingAppVer,

    #[error("CATEGORY parameter not found")]
    MissingCategory,

    #[error("CONTENT_ID parameter not found")]
    MissingContentId,

    #[error("TITLE parameter not found")]
    MissingTitle,

    #[error("TITLE_ID parameter not found")]
    MissingTitleId,

    #[error("APP_VER and CONTENT_VER parameter not found")]
    MissingVersion,
}
