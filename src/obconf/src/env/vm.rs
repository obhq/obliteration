use core::num::NonZero;

/// Provides boot information when booting on a Virtual Machine.
#[repr(C)]
pub struct Vm {
    /// Address of [ConsoleMemory].
    pub console: usize,
    /// Page size on the host.
    pub host_page_size: NonZero<usize>,
}

/// Layout of console memory.
///
/// The sequence of operations on a console memory is per-cpu. The kernel will start each log by:
///
/// 1. Write [`Self::msg_len`] then [`Self::msg_addr`].
/// 2. Repeat step 1 until the whole message has been written.
/// 3. Write [`Self::commit`].
///
/// Beware that each write to [`Self::msg_len`] except the last one may not cover the full message.
/// The consequence of this is [`Self::msg_addr`] may point to an incomplete UTF-8 byte sequence.
/// That mean you should buffer the message until [`Self::commit`] has been written before validate
/// if it is a valid UTF-8.
#[cfg(feature = "virt")]
#[repr(C)]
pub struct ConsoleMemory {
    pub msg_len: NonZero<usize>,
    pub msg_addr: usize,
    pub commit: ConsoleType,
}

/// Type of console message.
#[cfg(feature = "virt")]
#[repr(u8)]
#[derive(Clone, Copy, num_enum::IntoPrimitive, num_enum::TryFromPrimitive)]
pub enum ConsoleType {
    Info,
    Warn,
    Error,
}
