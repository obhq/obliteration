/// Type of console message.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum MsgType {
    Info,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            v if v == Self::Info as u8 => Self::Info,
            _ => return None,
        })
    }
}
