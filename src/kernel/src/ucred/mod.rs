pub use self::auth::*;
pub use self::privilege::*;
use crate::errno::{Errno, EPERM};
use std::num::NonZeroI32;
use thiserror::Error;

mod auth;
mod privilege;

/// An implementation of `ucred` structure.
#[derive(Debug, Clone)]
pub struct Ucred {
    effective_uid: i32, // cr_uid
    real_uid: i32,      // cr_ruid
    groups: Vec<i32>,   // cr_groups + cr_ngroups
    auth: AuthInfo,
}

impl Ucred {
    pub fn new(effective_uid: i32, real_uid: i32, mut groups: Vec<i32>, auth: AuthInfo) -> Self {
        assert!(effective_uid >= 0);
        assert!(real_uid >= 0);
        assert!(!groups.is_empty()); // Must have primary group.

        groups[1..].sort_unstable(); // The first one must be primary group.

        Self {
            effective_uid,
            real_uid,
            groups,
            auth,
        }
    }

    pub fn effective_uid(&self) -> i32 {
        self.effective_uid
    }

    pub fn auth(&self) -> &AuthInfo {
        &self.auth
    }

    /// See `groupmember` on the PS4 for a reference.
    pub fn is_member(&self, gid: i32) -> bool {
        if self.groups[0] == gid {
            return true;
        }

        self.groups[1..].binary_search(&gid).is_ok()
    }

    /// See `sceSblACMgrIsWebcoreProcess` on the PS4 for a reference.
    pub fn is_webcore_process(&self) -> bool {
        // TODO: Refactor this for readability.
        let id = self.auth.paid.get().wrapping_add(0xc7ffffffeffffffc);
        (id < 0xf) && ((0x6001 >> (id & 0x3f) & 1) != 0)
    }

    /// See `sceSblACMgrIsDiskplayeruiProcess` on the PS4 for a reference.
    pub fn is_diskplayerui_process(&self) -> bool {
        self.auth.paid.get() == 0x380000001000000f || self.auth.paid.get() == 0x3800000010000013
    }

    /// See `sceSblACMgrIsNongameUcred` on the PS4 for a reference.
    pub fn is_nongame(&self) -> bool {
        self.auth.caps.is_nongame()
    }

    /// See `sceSblACMgrIsSystemUcred` on the PS4 for a reference.
    pub fn is_system(&self) -> bool {
        self.auth.caps.is_system()
    }

    pub fn is_unk1(&self) -> bool {
        self.auth.caps.is_unk1() && self.auth.attrs.is_unk1()
    }

    pub fn is_unk2(&self) -> bool {
        self.auth.caps.is_unk1() && self.auth.attrs.is_unk2()
    }

    /// See `priv_check_cred` on the PS4 for a reference.
    pub fn priv_check(&self, p: Privilege) -> Result<(), PrivilegeError> {
        // TODO: Check suser_enabled.
        self.prison_priv_check()?;

        let r = match p {
            Privilege::MAXFILES
            | Privilege::PROC_SETLOGIN
            | Privilege::SCE680
            | Privilege::SCE683
            | Privilege::SCE686 => self.is_system(),
            v => todo!("priv_check_cred(cred, {v})"),
        };

        if r {
            Ok(())
        } else {
            Err(PrivilegeError::NoPrivilege)
        }
    }

    /// See `prison_priv_check` on the PS4 for a reference.
    fn prison_priv_check(&self) -> Result<(), PrivilegeError> {
        // TODO: Implement this.
        Ok(())
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
