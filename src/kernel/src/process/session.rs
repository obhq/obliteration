use std::num::NonZeroI32;

/// An implementation of `session` structure.
#[derive(Debug)]
pub struct VSession {
    id: NonZeroI32, // s_sid
    login: String,  // s_login
}

impl VSession {
    pub fn new(id: NonZeroI32, login: String) -> Self {
        Self { id, login }
    }

    pub fn set_login<V: Into<String>>(&mut self, v: V) {
        self.login = v.into();
    }
}
