// SPDX-License-Identifier: MIT OR Apache-2.0
use std::io::{Error, ErrorKind, Read, Write};
use std::net::TcpStream;

/// Encapsulate a debugger connection.
pub struct DebugClient {
    sock: TcpStream,
    buf: Vec<u8>,
    next: usize,
}

impl DebugClient {
    pub(super) fn new(sock: TcpStream) -> Self {
        Self {
            sock,
            buf: Vec::new(),
            next: 0,
        }
    }

    #[cfg(unix)]
    pub fn socket(&self) -> std::os::fd::RawFd {
        std::os::fd::AsRawFd::as_raw_fd(&self.sock)
    }

    #[cfg(windows)]
    pub fn socket(&self) -> std::os::windows::io::RawSocket {
        std::os::windows::io::AsRawSocket::as_raw_socket(&self.sock)
    }

    pub fn read(&mut self) -> Result<u8, Error> {
        // Fill the buffer if needed.
        while self.next == self.buf.len() {
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
                return Err(Error::from(ErrorKind::UnexpectedEof));
            }

            self.buf.extend_from_slice(&buf[..len]);
        }

        // Get byte.
        let b = self.buf[self.next];

        self.next += 1;

        Ok(b)
    }
}

impl gdbstub::conn::Connection for DebugClient {
    type Error = std::io::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.write_all(std::slice::from_ref(&byte))
    }

    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Self::Error> {
        while !buf.is_empty() {
            let written = match Write::write(&mut self.sock, buf) {
                Ok(v) => v,
                Err(e) if matches!(e.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) => {
                    continue;
                }
                Err(e) => return Err(e),
            };

            if written == 0 {
                return Err(std::io::Error::from(ErrorKind::WriteZero));
            }

            buf = &buf[written..];
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        while let Err(e) = Write::flush(&mut self.sock) {
            if !matches!(e.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) {
                return Err(e);
            }
        }

        Ok(())
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        self.sock.set_nodelay(true)
    }
}
