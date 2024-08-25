/// Layout of console memory.
///
/// The sequence of operations on a console memory is per-cpu. The kernel will start each log by:
///
/// 1. Write [`Self::msg_len`] then [`Self::msg_addr`].
/// 2. Repeat step 1 until the whole message has been written.
/// 3. Write [`Self::commit`].
#[repr(C)]
pub struct Memory {
    pub msg_len: usize,
    pub msg_addr: usize,
    pub commit: MsgType,
}

/// Type of console message.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum MsgType {
    Info,
    Warn,
    Error,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        let v = match v {
            v if v == Self::Info as u8 => Self::Info,
            v if v == Self::Warn as u8 => Self::Warn,
            v if v == Self::Error as u8 => Self::Error,
            _ => return None,
        };

        Some(v)
    }
}
