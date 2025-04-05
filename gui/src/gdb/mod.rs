// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::handler::*;

use self::client::ClientDispatcher;
use self::state::SessionState;
use thiserror::Error;

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
    ) -> impl GdbDispatcher + 'a {
        self.req.extend_from_slice(data);

        ClientDispatcher::new(self, h)
    }
}

/// Provides method to dispatch debug operations.
pub trait GdbDispatcher {
    /// The returned response can be empty if this pump does not produce any response.
    fn pump(&mut self) -> Result<Option<impl AsRef<[u8]> + '_>, GdbError>;
}

/// Represents an error when [`GdbDispatcher`] fails.
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
}
