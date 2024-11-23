// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::client::*;

use std::ffi::CString;
use std::net::{TcpListener, ToSocketAddrs};
use thiserror::Error;

mod client;

/// TCP listener to accept a debugger connection.
pub struct DebugServer {
    addr: CString,
    sock: TcpListener,
}

impl DebugServer {
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self, StartDebugServerError> {
        let sock = TcpListener::bind(addr).map_err(StartDebugServerError::BindFailed)?;
        let addr = sock
            .local_addr()
            .map_err(StartDebugServerError::GetAddrFailed)?;

        Ok(Self {
            addr: CString::new(addr.to_string()).unwrap(),
            sock,
        })
    }

    pub fn accept(&self) -> std::io::Result<DebugClient> {
        let (sock, _) = self.sock.accept()?;

        Ok(DebugClient::new(sock))
    }
}

#[derive(Debug, Error)]
pub enum StartDebugServerError {
    #[error("couldn't bind to the specified address")]
    BindFailed(#[source] std::io::Error),

    #[error("couldn't get server address")]
    GetAddrFailed(#[source] std::io::Error),
}
