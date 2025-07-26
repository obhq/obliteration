/// Contains states for a GDB remote session.
#[derive(Default)]
pub struct SessionState {
    no_ack: Option<bool>,
}

impl SessionState {
    pub fn no_ack(&self) -> Option<bool> {
        self.no_ack
    }

    pub fn parse_start_no_ack_mode(&mut self, res: &mut Vec<u8>) {
        self.no_ack = Some(false);

        res.extend_from_slice(b"OK");
    }

    pub fn parse_ack_no_ack(&mut self) {
        self.no_ack = Some(true);
    }

    pub fn parse_supported(&mut self, req: &[u8], res: &mut Vec<u8>) {
        // Push features that we always supported.
        res.extend_from_slice(b"QStartNoAckMode+");

        // Parse GDB features.
        let req = match req.strip_prefix(b":") {
            Some(v) => v,
            None => return,
        };

        for feat in req.split(|&b| b == b';') {
            if let Some(_) = feat.strip_prefix(b"xmlRegisters=") {
                res.extend_from_slice(b";xmlRegisters=");
                res.extend_from_slice(if cfg!(target_arch = "aarch64") {
                    b"arm"
                } else if cfg!(target_arch = "x86_64") {
                    b"i386"
                } else {
                    todo!()
                });
            } else {
                todo!("{}", String::from_utf8_lossy(feat));
            }
        }
    }
}
