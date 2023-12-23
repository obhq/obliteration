use super::{VProc, VSession};
use std::{num::NonZeroI32, sync::Arc};

/// An implementation of `pgrp` struct.
#[derive(Debug)]
pub struct VProcGroup {
    id: NonZeroI32,                 // pg_id
    session: Option<Arc<VSession>>, // pg_session
    leader: Arc<VProc>,             // pg_leader
}

impl VProcGroup {
    pub fn new(id: NonZeroI32, session: VSession, leader: &Arc<VProc>) -> Self {
        Self {
            id,
            session: Some(Arc::new(session)),
            leader: leader.clone(),
        }
    }

    pub fn session(&self) -> Option<&Arc<VSession>> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut Arc<VSession>> {
        self.session.as_mut()
    }

    pub fn leader(&self) -> &Arc<VProc> {
        &self.leader
    }
}
