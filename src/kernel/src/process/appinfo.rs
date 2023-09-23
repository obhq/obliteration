use crate::errno::{Errno, EINVAL};
use thiserror::Error;

/// An implementation of `appinfo` structure on the PS4.
#[derive(Debug)]
pub struct AppInfo {
    unk1: u32, // 0x02 = ET_SCE_REPLAY_EXEC
    title_id: String,
}

impl AppInfo {
    pub fn new() -> Self {
        Self {
            unk1: 0,
            title_id: String::new(),
        }
    }

    pub fn unk1(&self) -> u32 {
        self.unk1
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<(), AppInfoReadError> {
        if buf.len() >= 73 {
            return Err(AppInfoReadError::UnknownBuffer);
        } else if buf.len() != 72 {
            todo!("appinfo with size != 72");
        }

        // TODO: Right now we don't know how appinfo is structured but it seems like it is safe to
        // fill it with zeroes.
        buf.fill(0);

        Ok(())
    }
}

/// Represents an error when [`AppInfo::read()`] is failed.
#[derive(Debug, Error)]
pub enum AppInfoReadError {
    #[error("size of the buffer is not recognized")]
    UnknownBuffer,
}

impl Errno for AppInfoReadError {
    fn errno(&self) -> std::num::NonZeroI32 {
        match self {
            Self::UnknownBuffer => EINVAL,
        }
    }
}
