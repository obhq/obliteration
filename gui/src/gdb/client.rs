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

    fn get_checksum(data: &[u8]) -> u8 {
        let mut r = 0u8;

        for &b in data {
            r = r.wrapping_add(b);
        }

        r
    }
}

impl<'a, H: GdbHandler> GdbDispatcher for ClientDispatcher<'a, H> {
    fn pump(&mut self) -> Result<Option<impl AsRef<[u8]> + '_>, GdbError> {
        // Check if GDB packet.
        let req = &mut self.session.req;
        let res = &mut self.session.res;
        let state = &mut self.session.state;

        match req.first().copied() {
            Some(b'$') => (),
            Some(b'+') => {
                match state.no_ack() {
                    Some(true) => return Err(GdbError::UnexpectedAck),
                    Some(false) => state.parse_ack_no_ack(),
                    None => (),
                }

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
        let data = <[u8; 2]>::try_from(&cmd[(cmd.len() - 2)..]).unwrap();
        let mut checksum = 0;

        if let Err(e) = hex::decode_to_slice(&data, std::slice::from_mut(&mut checksum)) {
            match state.no_ack() {
                Some(true) => return Err(GdbError::DecodeChecksum(data, e)),
                Some(false) => return Err(GdbError::MissingAck),
                None => {
                    // TODO: Should we consider this as an invalid packet instead?
                    res.push(b'-'); // Request retransmission.
                    return Ok(res.drain(..).into());
                }
            }
        }

        // Calculate expected checksum.
        let data = &cmd[1..(cmd.len() - 3)];
        let expect = Self::get_checksum(data);

        if checksum != expect {
            match state.no_ack() {
                Some(true) => return Err(GdbError::InvalidChecksum(checksum, expect)),
                Some(false) => return Err(GdbError::MissingAck),
                None => {
                    res.push(b'-'); // Request retransmission.
                    return Ok(res.drain(..).into());
                }
            }
        }

        // Push response prefix.
        match state.no_ack() {
            Some(true) => (),
            Some(false) => return Err(GdbError::MissingAck),
            None => res.push(b'+'),
        }

        res.push(b'$');

        // Execute command.
        let off = res.len();

        match data {
            b"QStartNoAckMode" => state.parse_start_no_ack_mode(res),
            v => todo!("{}", String::from_utf8_lossy(v)),
        }

        // Push checksum.
        let mut checksum = [0u8; 2];

        hex::encode_to_slice([Self::get_checksum(&res[off..])], &mut checksum).unwrap();

        res.push(b'#');
        res.extend_from_slice(&checksum);

        Ok(res.drain(..).into())
    }
}
