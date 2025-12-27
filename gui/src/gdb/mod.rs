// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::handler::*;

use self::client::ClientDispatcher;
use self::state::SessionState;
use thiserror::Error;

/// Match byte slices against patterns, similar to a match statement.
///
/// Supports:
/// - Exact matches: `"qProcessInfo" => { ... }`
/// - Prefix matches with capture: `["qSupported", rest] => { ... }`
/// - Default case: `_ => { ... }`
macro_rules! match_bytes {
    ($data:expr, $($pattern:tt => $body:expr),+ $(,)?) => {{
        let __data = $data;
        match_bytes!(@arms __data, $($pattern => $body),+)
    }};

    // Terminal case with default
    (@arms $data:ident, _ => $body:expr) => {
        $body
    };

    // Exact match followed by more patterns
    (@arms $data:ident, $lit:literal => $body:expr, $($rest:tt)+) => {
        if $data == $lit.as_bytes() {
            $body
        } else {
            match_bytes!(@arms $data, $($rest)+)
        }
    };

    // Prefix match followed by more patterns
    (@arms $data:ident, [$prefix:literal, $capture:ident] => $body:expr, $($rest:tt)+) => {
        if let Some($capture) = $data.strip_prefix($prefix.as_bytes()) {
            $body
        } else {
            match_bytes!(@arms $data, $($rest)+)
        }
    };

    // Final exact match (no default)
    (@arms $data:ident, $lit:literal => $body:expr) => {
        if $data == $lit.as_bytes() {
            $body
        } else {
            todo!("{}", String::from_utf8_lossy($data))
        }
    };

    // Final prefix match (no default)
    (@arms $data:ident, [$prefix:literal, $capture:ident] => $body:expr) => {
        if let Some($capture) = $data.strip_prefix($prefix.as_bytes()) {
            $body
        } else {
            todo!("{}", String::from_utf8_lossy($data))
        }
    };
}

mod client;
mod handler;
mod state;

/// Represents a GDB remote session.
///
/// This type requires the client to be compatible with GDB >= 5.0.
#[derive(Default)]
pub struct GdbSession {
    req: Vec<u8>,
    res: Vec<u8>,
    state: SessionState,
}

impl GdbSession {
    #[must_use]
    pub fn dispatch_client<'a, H: GdbHandler>(
        &'a mut self,
        data: &[u8],
        h: &'a mut H,
    ) -> ClientDispatcher<'a, H> {
        self.req.extend_from_slice(data);

        ClientDispatcher::new(self, h)
    }
}

/// Represents an error when [ClientDispatcher::pump()] fails.
#[derive(Debug, Error)]
pub enum GdbError {
    #[error("unknown packet prefix {0:#x}")]
    UnknownPacketPrefix(u8),

    #[error("unexpected acknowledgment packet from GDB")]
    UnexpectedAck,

    #[error("missing acknowledgment packet from GDB")]
    MissingAck,

    #[error("couldn't decode checksum {0:?}")]
    DecodeChecksum([u8; 2], #[source] hex::FromHexError),

    #[error("invalid checksum (expect {1}, got {0})")]
    InvalidChecksum(u8, u8),

    #[error("couldn't parse '{0}'")]
    Parse(&'static str, #[source] Box<dyn std::error::Error>),
}
