/// Layout of console memory.
///
/// The sequence of operations on a console memory is per-cpu. The kernel will start each log by:
///
/// 1. Write [`Self::file_len`] then [`Self::file_addr`].
/// 2. Write [`Self::msg_len`] then [`Self::msg_addr`].
/// 3. Repeat step 2 until the whole message has been written.
/// 4. Write [`Self::commit`].
#[repr(C)]
pub struct Memory {
    pub file_len: usize,
    pub file_addr: usize,
    pub msg_len: usize,
    pub msg_addr: usize,
    pub commit: Commit,
}

/// Struct to commit a log.
#[repr(transparent)]
pub struct Commit(u32);

impl Commit {
    /// # Panics
    /// If `line` greater than 0xffffff.
    pub fn new(ty: MsgType, line: u32) -> Self {
        assert!(line <= 0xffffff);

        Self((ty as u32) << 24 | line)
    }

    pub fn parse(raw: u32) -> Option<(MsgType, u32)> {
        let line = raw & 0xffffff;
        let ty = match (raw >> 24) as u8 {
            v if v == MsgType::Info as u8 => MsgType::Info,
            _ => return None,
        };

        Some((ty, line))
    }
}

/// Type of console message.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum MsgType {
    Info,
}
