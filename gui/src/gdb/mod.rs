// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::handler::*;

use self::client::ClientHandler;
use thiserror::Error;

mod client;
mod handler;

/// Contains states for a GDB remote session.
#[derive(Default)]
pub struct GdbSession {
    req: Vec<u8>,
    res: Vec<u8>,
}

impl GdbSession {
    #[must_use]
    pub fn dispatch_client<'a, H: GdbHandler>(
        &'a mut self,
        data: &[u8],
        h: &'a mut H,
    ) -> impl GdbExecutor + 'a {
        self.req.extend_from_slice(data);

        ClientHandler::new(self, h)
    }
}

/// Provides method to execute debug operations.
pub trait GdbExecutor {
    /// The returned response can be empty if this pump does not produce any response.
    fn pump(&mut self) -> Result<Option<impl AsRef<[u8]> + '_>, GdbError>;
}

/// Represents an error when [`GdbExecutor`] fails.
#[derive(Debug, Error)]
pub enum GdbError {}
