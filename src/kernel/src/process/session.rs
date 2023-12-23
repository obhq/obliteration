use crate::{fs::Vnode, tty::Tty};
use gmtx::*;
use std::{num::NonZeroI32, sync::Arc};

/// An implementation of `session` structure.
#[derive(Debug)]
pub struct VSession {
    id: NonZeroI32,               // s_sid
    login: Gutex<String>,         // s_login
    vnode: Option<Vnode>,         // s_ttyvp
    tty: Gutex<Option<Arc<Tty>>>, // s_ttyp
    gg: Arc<GutexGroup>,
}

impl VSession {
    pub fn new(id: NonZeroI32, login: String) -> Self {
        let gg = GutexGroup::new();

        Self {
            id,
            login: gg.spawn(login),
            vnode: None,
            tty: gg.spawn(None),
            gg,
        }
    }

    pub fn login_mut(&self) -> GutexWriteGuard<'_, String> {
        self.login.write()
    }

    pub fn vnode(&self) -> Option<&Vnode> {
        self.vnode.as_ref()
    }

    pub fn tty(&self) -> GutexReadGuard<'_, Option<Arc<Tty>>> {
        self.tty.read()
    }

    pub fn tty_mut(&self) -> GutexWriteGuard<'_, Option<Arc<Tty>>> {
        self.tty.write()
    }
}
