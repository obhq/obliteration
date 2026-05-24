use super::{GdbError, GdbHandler, GdbSession};
use arrayvec::ArrayVec;

/// Implementation of [`GdbDispatcher`] to dispatch requests from GDB client.
pub struct ClientDispatcher<'a, H> {
    session: &'a mut GdbSession,
    handler: &'a mut H,
}

impl<'a, H: GdbHandler> ClientDispatcher<'a, H> {
    pub fn new(session: &'a mut GdbSession, handler: &'a mut H) -> Self {
        Self { session, handler }
    }

    pub async fn pump(
        &mut self,
    ) -> Result<Option<(ArrayVec<u8, 2>, Vec<u8>, ArrayVec<u8, 3>)>, GdbError> {
        // Check if GDB packet.
        let req = &mut self.session.req;
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

                return Ok(Some(Default::default()));
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

        if let Err(e) = hex::decode_to_slice(data, std::slice::from_mut(&mut checksum)) {
            match state.no_ack() {
                Some(true) => return Err(GdbError::DecodeChecksum(data, e)),
                Some(false) => return Err(GdbError::MissingAck),
                None => {
                    // TODO: Should we consider this as an invalid packet instead?
                    let mut h = ArrayVec::new();

                    h.push(b'-'); // Request retransmission.

                    return Ok(Some((h, Vec::new(), ArrayVec::new())));
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
                    let mut h = ArrayVec::new();

                    h.push(b'-'); // Request retransmission.

                    return Ok(Some((h, Vec::new(), ArrayVec::new())));
                }
            }
        }

        // Push response prefix.
        let mut head = ArrayVec::new();

        match state.no_ack() {
            Some(true) => (),
            Some(false) => return Err(GdbError::MissingAck),
            None => head.push(b'+'),
        }

        // Execute command.
        let body;

        macro_rules! parse {
            () => {
                todo!("{}", String::from_utf8_lossy(data));
            };
            ($cmd:literal => $body:expr, $($rem:tt)*) => {
                if data == $cmd.as_bytes() {
                    match $body {
                        Ok(v) => body = v,
                        Err(e) => return Err(GdbError::Parse($cmd, e)),
                    }
                } else {
                    parse!($($rem)*);
                }
            };
            ($prefix:literal | $data:ident => $body:expr, $($rem:tt)*) => {
                if let Some($data) = data.strip_prefix($prefix.as_bytes()) {
                    match $body {
                        Ok(v) => body = v,
                        Err(e) => return Err(GdbError::Parse($prefix, e)),
                    }
                } else {
                    parse!($($rem)*);
                }
            }
        }

        parse! {
            // Queries the reason the target halted. Defined on the Packets page (search for "'?'"
            // near the top of the packet list).
            // See https://sourceware.org/gdb/current/onlinedocs/gdb.html/Packets.html
            "?" => state.parse_stop_reason(self.handler).await,
            "c" | data => state.parse_continue(data, self.handler),
            "jThreadsInfo" => Ok(Some(Vec::new())),
            "m" | data => state.parse_read_memory(data, self.handler, false).await,
            // https://sourceware.org/gdb/current/onlinedocs/gdb.html/Packets.html
            "p" | data => state.parse_read_register(data, self.handler).await,
            // https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html#index-qC-packet
            "qC" => state.parse_current_thread(),
            // I think this does not worth for additional complexity on our side so we don't support
            // this. See https://lldb.llvm.org/resources/lldbgdbremote.html#qenableerrorstrings for
            // more details.
            "QEnableErrorStrings" => Ok(Some(Vec::new())),
            // https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html#index-qfThreadInfo-packet
            "qfThreadInfo" => state.parse_first_thread_info(self.handler),
            // https://lldb.llvm.org/resources/lldbgdbremote.html#qhostinfo
            "qHostInfo" => state.parse_host_info(),
            // https://lldb.llvm.org/resources/lldbgdbremote.html#qlistthreadsinstopreply
            "QListThreadsInStopReply" => state.parse_enable_threads_in_stop_reply(),
            // The VMM already relocated the kernel.
            "qOffsets" => Ok(Some(Vec::new())),
            // https://lldb.llvm.org/resources/lldbgdbremote.html#qregisterinfo-hex-reg-id
            "qRegisterInfo" | reg => state.parse_register_info(reg),
            // https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html#index-qsThreadInfo-packet
            "qsThreadInfo" => state.parse_subsequent_thread_info(),
            // TODO: What is this?
            "qStructuredDataPlugins" => Ok(Some(Vec::new())),
            // This does not useful to us. See
            // https://lldb.llvm.org/resources/lldbgdbremote.html#qprocessinfo for more details.
            "qProcessInfo" => Ok(Some(Vec::new())),
            "QStartNoAckMode" => state.parse_start_no_ack_mode(),
            // It is unclear if qSupported can sent from GDB without additional payload.
            "qSupported" | rest => state.parse_supported(rest),
            // https://lldb.llvm.org/resources/lldbgdbremote.html#qthreadsuffixsupported
            "QThreadSuffixSupported" => state.parse_thread_suffix_supported(),
            // TODO: https://github.com/obhq/obliteration/issues/1398
            "qVAttachOrWaitSupported" => Ok(Some(Vec::new())),
            "vCont?" => state.parse_vcont(),
            "x" | data => state.parse_read_memory(data, self.handler, true).await,
        }

        // Get checksum.
        let mut tail = ArrayVec::new();
        let body = match body {
            Some(v) => v,
            None => return Ok(Some((head, Vec::new(), tail))),
        };

        head.push(b'$');
        tail.push(b'#');
        tail.push(0);
        tail.push(0);

        hex::encode_to_slice([Self::get_checksum(&body)], &mut tail[1..]).unwrap();

        Ok(Some((head, body, tail)))
    }

    fn get_checksum(data: &[u8]) -> u8 {
        let mut r = 0u8;

        for &b in data {
            r = r.wrapping_add(b);
        }

        r
    }
}
