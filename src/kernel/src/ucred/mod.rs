pub use self::auth::*;
pub use self::privilege::*;

mod auth;
mod privilege;

/// An implementation of `ucred` structure.
#[derive(Debug)]
pub struct Ucred {
    auth: AuthInfo,
}

impl Ucred {
    pub fn new(auth: AuthInfo) -> Self {
        Self { auth }
    }

    pub fn auth(&self) -> &AuthInfo {
        &self.auth
    }

    /// See `sceSblACMgrIsWebcoreProcess` on the PS4 for a reference.
    pub fn is_webcore_process(&self) -> bool {
        // TODO: Implement this.
        false
    }

    /// See `sceSblACMgrIsDiskplayeruiProcess` on the PS4 for a reference.
    pub fn is_diskplayerui_process(&self) -> bool {
        // TODO: Implement this.
        false
    }

    /// See `sceSblACMgrIsNongameUcred` on the PS4 for a reference.
    pub fn is_nongame(&self) -> bool {
        // TODO: Implement this.
        false
    }

    /// An implementation of `priv_check_cred`.
    pub fn has_priv(&self, p: Privilege) -> bool {
        match p {
            Privilege::SCE686 => false,
            v => todo!("priv_check_cred(cred, {v})"),
        }
    }
}
