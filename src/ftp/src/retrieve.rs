use crate::{FtpClient, Reply};
use std::io::{IoSliceMut, Read};
use std::mem::ManuallyDrop;
use std::net::{Shutdown, TcpStream};
use thiserror::Error;

/// Encapsulates a data connection for [`FtpClient::retrieve()`].
pub struct Retrieve<'a> {
    client: &'a mut FtpClient,
    data: TcpStream,
}

impl<'a> Retrieve<'a> {
    pub(super) fn new(client: &'a mut FtpClient, data: TcpStream) -> Self {
        Self { client, data }
    }

    pub fn close(self) -> Result<(), CloseError> {
        ManuallyDrop::new(self).close_data()
    }

    fn close_data(&mut self) -> Result<(), CloseError> {
        // Shutdown the data connection.
        if let Err(e) = self.data.shutdown(Shutdown::Both) {
            return Err(CloseError::ShutdownFailed(e));
        }

        // Wait for 2xx reply.
        match self.client.read_reply() {
            Ok(v) => {
                if !v.is_positive_completion() {
                    return Err(CloseError::UnexpectedReply(v));
                }
            }
            Err(e) => return Err(CloseError::ReadReplyFailed(e)),
        }

        Ok(())
    }
}

impl<'a> Read for Retrieve<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.data.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.data.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.data.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.data.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.data.read_exact(buf)
    }
}

impl<'a> Drop for Retrieve<'a> {
    fn drop(&mut self) {
        self.close_data().unwrap();
    }
}

/// Represents an error for [`Retrieve::close()`]
#[derive(Debug, Error)]
pub enum CloseError {
    #[error("cannot shutdown the data connection")]
    ShutdownFailed(#[source] std::io::Error),

    #[error("cannot read the reply")]
    ReadReplyFailed(#[source] crate::ReadReplyError),

    #[error("unexpected reply ({0})")]
    UnexpectedReply(Reply),
}
