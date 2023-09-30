pub use self::auth::*;
pub use self::privilege::*;

use crate::errno::{Errno, EPERM};
use std::num::NonZeroI32;
use thiserror::Error;

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
    pub fn priv_check(&self, p: Privilege) -> Result<(), PrivilegeError> {
        // TODO: Implement this.
        let r = match p {
            Privilege::SCE680 | Privilege::SCE683 => true,
            Privilege::MAXFILES | Privilege::SCE686 => false,
            v => todo!("priv_check_cred(cred, {v})"),
        };

        if r {
            Ok(())
        } else {
            Err(PrivilegeError::NoPrivilege)
        }
    }
}

/// Represents an error when [`Ucred::priv_check()`] is failed.
#[derive(Debug, Error)]
pub enum PrivilegeError {
    #[error("the current credential does not have the specified privilege")]
    NoPrivilege,
}

impl Errno for PrivilegeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NoPrivilege => EPERM,
        }
    }
}
