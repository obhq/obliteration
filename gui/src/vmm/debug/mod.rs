// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

use gdbstub::conn::Connection;
use std::io::{Read, Write};
use std::net::TcpStream;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;

/// Implementation of [`Connection`] using `select` system call to check if data available.
pub struct Client {
    sock: TcpStream,
    buf: Vec<u8>,
    next: usize,
}

impl Client {
    pub fn new(sock: TcpStream) -> Self {
        Self {
            sock,
            buf: Vec::new(),
            next: 0,
        }
    }

    pub fn read(&mut self) -> Result<u8, std::io::Error> {
        // Fill the buffer if needed.
        while self.next == self.buf.len() {
            use std::io::ErrorKind;

            // Clear previous data.
            self.buf.clear();
            self.next = 0;

            // Read.
            let mut buf = [0; 1024];
            let len = match self.sock.read(&mut buf) {
                Ok(v) => v,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };

            if len == 0 {
                return Err(std::io::Error::from(ErrorKind::UnexpectedEof));
            }

            self.buf.extend_from_slice(&buf[..len]);
        }

        // Get byte.
        let b = self.buf[self.next];

        self.next += 1;

        Ok(b)
    }
}

impl Connection for Client {
    type Error = std::io::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        Write::write_all(&mut self.sock, std::slice::from_ref(&byte))
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        Write::write_all(&mut self.sock, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Write::flush(&mut self.sock)
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        self.sock.set_nodelay(true)
    }
}
