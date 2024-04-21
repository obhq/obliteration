use super::Pid;
use gmtx::{Gutex, GutexGroup};
use std::sync::Arc;

/// An implementation of `session` structure.
#[derive(Debug)]
pub struct VSession {
    id: Pid,              // s_sid
    login: Gutex<String>, // s_login
}

impl VSession {
    pub fn new(id: Pid, login: String) -> Arc<Self> {
        let gg = GutexGroup::new();

        Arc::new(Self {
            id,
            login: gg.spawn(login),
        })
    }

    pub fn set_login<V: Into<String>>(&self, v: V) {
        *self.login.write() = v.into();
    }
}
