use std::{num::NonZeroI32, sync::Arc};

use crate::{fs::Vnode, tty::Tty};

/// An implementation of `session` structure.
#[derive(Debug)]
pub struct VSession {
    id: NonZeroI32,        // s_sid
    login: String,         // s_login
    vnode: Option<Vnode>,  // s_ttyvp
    tty: Option<Arc<Tty>>, // s_ttyp
    refcount: u32,         // s_count
}

impl VSession {
    pub fn new(id: NonZeroI32, login: String) -> Self {
        Self {
            id,
            login,
            vnode: None,
            tty: None,
            refcount: 1,
        }
    }

    pub fn set_login<V: Into<String>>(&mut self, v: V) {
        self.login = v.into();
    }
}
