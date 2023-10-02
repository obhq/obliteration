use super::VSession;
use std::num::NonZeroI32;

/// An implementation of `pgrp` struct.
#[derive(Debug)]
pub struct VProcGroup {
    id: NonZeroI32,    // pg_id
    session: VSession, // pg_session
}

impl VProcGroup {
    pub fn new(id: NonZeroI32, session: VSession) -> Self {
        Self { id, session }
    }

    pub fn session_mut(&mut self) -> &mut VSession {
        &mut self.session
    }
}
