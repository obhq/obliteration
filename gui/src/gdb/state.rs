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
}
