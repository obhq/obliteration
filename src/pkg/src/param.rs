use std::error::Error;
use std::fmt::{Display, Formatter};
use util::mem::{read_u16_be, read_u16_le, read_u32_be, read_u32_le};

macro_rules! utf8 {
    ($num:ident, $value:ident, $format:ident) => {{
        if $format != 0x0402 {
            return Err(ReadError::InvalidValueFormat($num));
        } else if let Ok(v) = std::str::from_utf8(&$value[..($value.len() - 1)]) {
            v.into()
        } else {
            return Err(ReadError::InvalidValue($num));
        }
    }};
}

macro_rules! integer {
    ($num:ident, $value:ident, $format:ident) => {{
        if $format != 0x0404 {
            return Err(ReadError::InvalidValueFormat($num));
        } else if $value.len() != 4 {
            return Err(ReadError::InvalidValue($num));
        } else {
            read_u32_le($value.as_ptr(), 0)
        }
    }};
}

// https://www.psdevwiki.com/ps4/Param.sfo#Internal_Structure
pub struct Param {
    attribute: u32,
    system_ver: u32,
    title: String,
    title_id: String,
}

impl Param {
    pub fn read(raw: &[u8]) -> Result<Self, ReadError> {
        // Check minimum size.
        if raw.len() < 20 {
            return Err(ReadError::TooSmall);
        }

        // Check magic.
        let header = raw.as_ptr();
        let magic = read_u32_be(header, 0);

        if magic != 0x00505346 {
            return Err(ReadError::InvalidMagic);
        }

        // Read header.
        let key_table = read_u32_le(header, 8) as usize;
        let data_table = read_u32_le(header, 12) as usize;
        let entries = read_u32_le(header, 16) as usize;

        // Read entries.
        let mut attribute: Option<u32> = None;
        let mut system_ver: Option<u32> = None;
        let mut title: Option<String> = None;
        let mut title_id: Option<String> = None;

        for i in 0..entries {
            // Entry header.
            let offset = 20 + i * 16;
            let entry = match raw.get(offset..(offset + 16)) {
                Some(v) => v.as_ptr(),
                None => return Err(ReadError::TooSmall),
            };

            let key_offset = key_table + read_u16_le(entry, 0) as usize;
            let format = read_u16_be(entry, 2);
            let len = read_u32_le(entry, 4) as usize;
            let data_offset = data_table + read_u32_le(entry, 12) as usize;

            if len == 0 {
                return Err(ReadError::InvalidEntryHeader(i));
            }

            // Get key name.
            let key = match raw.get(key_offset..) {
                Some(v) => {
                    if let Some(i) = v.iter().position(|&b| b == 0) {
                        &v[..i]
                    } else {
                        return Err(ReadError::InvalidKeyOffset(i));
                    }
                }
                None => return Err(ReadError::InvalidKeyOffset(i)),
            };

            // Get value.
            let value = match raw.get(data_offset..(data_offset + len)) {
                Some(v) => v,
                None => return Err(ReadError::InvalidValueOffset(i)),
            };

            // Parse value.
            match key {
                b"ATTRIBUTE" => attribute = Some(integer!(i, value, format)),
                b"SYSTEM_VER" => system_ver = Some(integer!(i, value, format)),
                b"TITLE" => title = Some(utf8!(i, value, format)),
                b"TITLE_ID" => title_id = Some(utf8!(i, value, format)),
                _ => continue,
            }
        }

        // Check required values.
        let attribute = match attribute {
            Some(v) => v,
            None => return Err(ReadError::MissingAttribute),
        };

        let system_ver = match system_ver {
            Some(v) => v,
            None => return Err(ReadError::MissingSystemVer),
        };

        let title = match title {
            Some(v) => v,
            None => return Err(ReadError::MissingTitle),
        };

        let title_id = match title_id {
            Some(v) => v,
            None => todo!(),
        };

        Ok(Self {
            attribute,
            system_ver,
            title,
            title_id,
        })
    }

    pub fn title_id(&self) -> &str {
        self.title_id.as_ref()
    }

    pub fn title(&self) -> &str {
        self.title.as_ref()
    }
}

#[derive(Debug)]
pub enum ReadError {
    TooSmall,
    InvalidMagic,
    InvalidEntryHeader(usize),
    InvalidKeyOffset(usize),
    InvalidValueOffset(usize),
    InvalidValueFormat(usize),
    InvalidValue(usize),
    MissingAttribute,
    MissingSystemVer,
    MissingTitle,
    MissingTitleId,
}

impl Error for ReadError {}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::TooSmall => f.write_str("data too small"),
            Self::InvalidMagic => f.write_str("invalid magic"),
            Self::InvalidEntryHeader(i) => write!(f, "entry #{} has invalid header", i),
            Self::InvalidKeyOffset(i) => write!(f, "invalid key offset for entry #{}", i),
            Self::InvalidValueOffset(i) => write!(f, "invalid value offset for entry #{}", i),
            Self::InvalidValueFormat(i) => write!(f, "entry #{} has invalid value format", i),
            Self::InvalidValue(i) => write!(f, "entry #{} has invalid value", i),
            Self::MissingAttribute => f.write_str("ATTRIBUTE is not found"),
            Self::MissingSystemVer => f.write_str("SYSTEM_VER is not found"),
            Self::MissingTitle => f.write_str("TITLE is not found"),
            Self::MissingTitleId => f.write_str("TITLE_ID is not found"),
        }
    }
}
