use super::{GdbDispatcher, GdbError, GdbHandler, GdbSession};

/// Implementation of [`GdbDispatcher`] to dispatch requests from GDB client.
pub struct ClientDispatcher<'a, H> {
    session: &'a mut GdbSession,
    handler: &'a mut H,
}

impl<'a, H> ClientDispatcher<'a, H> {
    pub fn new(session: &'a mut GdbSession, handler: &'a mut H) -> Self {
        Self { session, handler }
    }
}

impl<'a, H: GdbHandler> GdbDispatcher for ClientDispatcher<'a, H> {
    fn pump(&mut self) -> Result<Option<impl AsRef<[u8]> + '_>, GdbError> {
        // Check if GDB packet.
        let req = &mut self.session.req;
        let res = &mut self.session.res;

        match req.first().copied() {
            Some(b'$') => (),
            Some(b'+') => {
                req.drain(..1);
                return Ok(res.drain(0..0).into());
            }
            Some(v) => return Err(GdbError::UnknownPacketPrefix(v)),
            None => return Ok(None),
        }

        // Check if packet complete.
        let cmd = match req
            .iter()
            .position(|&b| b == b'#')
            .map(|i| i + 3) // Two-digit checksum.
            .filter(|&e| e <= req.len())
        {
            Some(e) => req.drain(..e),
            None => return Ok(None),
        };

        // Parse checksum.
        let cmd = cmd.as_slice();
        let data = &cmd[(cmd.len() - 2)..];
        let mut checksum = 0;

        if hex::decode_to_slice(data, std::slice::from_mut(&mut checksum)).is_err() {
            // TODO: Should we consider this as an invalid packet instead?
            res.push(b'-'); // Request retransmission.
            return Ok(res.drain(..).into());
        }

        // Calculate expected checksum.
        let data = &cmd[1..(cmd.len() - 3)];
        let mut expect = 0u8;

        for &b in data {
            expect = expect.wrapping_add(b);
        }

        if checksum != expect {
            res.push(b'-'); // Request retransmission.
            return Ok(res.drain(..).into());
        }

        todo!()
    }
}
