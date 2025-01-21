use super::{GdbError, GdbExecutor, GdbHandler, GdbSession};

/// Implementation of [`GdbExecutor`] to execute requests from GDB client.
pub struct ClientHandler<'a, H> {
    session: &'a mut GdbSession,
    handler: &'a mut H,
}

impl<'a, H> ClientHandler<'a, H> {
    pub fn new(session: &'a mut GdbSession, handler: &'a mut H) -> Self {
        Self { session, handler }
    }
}

impl<'a, H: GdbHandler> GdbExecutor for ClientHandler<'a, H> {
    fn pump(&mut self) -> Result<Option<impl AsRef<[u8]> + '_>, GdbError> {
        Ok(Some(self.session.res.drain(..)))
    }
}
