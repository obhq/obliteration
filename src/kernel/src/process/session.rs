use std::num::NonZeroI32;
use std::sync::Arc;

use gmtx::{Gutex, GutexGroup};

/// An implementation of `session` structure.
#[derive(Debug)]
pub struct VSession {
    id: NonZeroI32,       // s_sid
    login: Gutex<String>, // s_login
}

impl VSession {
    pub fn new(id: NonZeroI32, login: String) -> Arc<Self> {
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
