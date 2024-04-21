use super::{Pid, VSession};
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::sync::Arc;

/// An implementation of `pgrp` struct.
#[derive(Debug)]
pub struct VProcGroup {
    id: Pid,                       // pg_id
    session: Gutex<Arc<VSession>>, // pg_session
}

impl VProcGroup {
    pub fn new(id: Pid, session: Arc<VSession>) -> Arc<Self> {
        let gg = GutexGroup::new();

        Arc::new(Self {
            id,
            session: gg.spawn(session),
        })
    }

    pub fn session(&self) -> GutexReadGuard<'_, Arc<VSession>> {
        self.session.read()
    }

    pub fn session_mut(&self) -> GutexWriteGuard<'_, Arc<VSession>> {
        self.session.write()
    }
}
