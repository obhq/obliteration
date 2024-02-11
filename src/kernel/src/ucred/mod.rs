use crate::errno::ESRCH;
use crate::errno::{Errno, EPERM};
use macros::Errno;
use std::borrow::Cow;
use thiserror::Error;

pub use self::auth::*;
pub use self::id::*;
pub use self::prison::*;
pub use self::privilege::*;

mod auth;
mod id;
mod prison;
mod privilege;

/// An implementation of `ucred` structure.
#[derive(Debug, Clone)]
pub struct Ucred {
    effective_uid: Uid,           // cr_uid
    real_uid: Uid,                // cr_ruid
    groups: Vec<Gid>,             // cr_groups + cr_ngroups
    prison: Cow<'static, Prison>, // cr_prison
    auth: AuthInfo,
}

impl Ucred {
    pub fn new(
        effective_uid: Uid,
        real_uid: Uid,
        mut groups: Vec<Gid>,
        prison: Cow<'static, Prison>,
        auth: AuthInfo,
    ) -> Self {
        assert!(!groups.is_empty()); // Must have primary group.

        groups[1..].sort_unstable(); // The first one must be primary group.

        Self {
            effective_uid,
            real_uid,
            groups,
            prison,
            auth,
        }
    }

    pub fn effective_uid(&self) -> Uid {
        self.effective_uid
    }

    pub fn real_uid(&self) -> Uid {
        self.real_uid
    }

    pub fn prison(&self) -> &Prison {
        &self.prison
    }

    pub fn auth(&self) -> &AuthInfo {
        &self.auth
    }

    /// See `groupmember` on the PS4 for a reference.
    pub fn is_member(&self, gid: Gid) -> bool {
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

    /// See `prison_check` on the PS4 for a reference.
    /// This function returns Ok(()) if the current credentials allow the holder to
    /// access objects associated with the credentials specified by `other`.
    pub fn can_access(&self, other: &Self) -> Result<(), PrisonCheckError> {
        if &self.prison == &other.prison || self.prison.is_child(&other.prison) {
            return Ok(());
        }

        Err(PrisonCheckError::InsuffiecientCredentials)
    }

    pub fn is_jailed(&self) -> bool {
        self.prison != Cow::Borrowed(&PRISON0)
    }

    /// See `priv_check_cred` on the PS4 for a reference.
    pub fn priv_check(&self, p: Privilege) -> Result<(), PrivilegeError> {
        // TODO: Check suser_enabled.
        self.prison_priv_check(p)?;

        let r = match p {
            Privilege::MAXFILES
            | Privilege::PROC_SETLOGIN
            | Privilege::SCE680
            | Privilege::SCE683
            | Privilege::SCE686 => self.is_system(),
            v => todo!("priv_check_cred({v})"),
        };

        if r {
            Ok(())
        } else {
            Err(PrivilegeError::NoPrivilege)
        }
    }

    /// See `prison_priv_check` on the PS4 for a reference.
    fn prison_priv_check(&self, p: Privilege) -> Result<(), PrivilegeError> {
        if !self.is_jailed() {
            return Ok(());
        }

        match p {
            Privilege::PROC_SETLOGIN
            | Privilege::VFS_READ
            | Privilege::VFS_WRITE
            | Privilege::VFS_ADMIN
            | Privilege::VFS_EXEC
            | Privilege::VFS_LOOKUP => Ok(()),
            _ => todo!("prison_priv_check({p})"),
        }
    }
}

/// Represents an error when [`Ucred::priv_check()`] is fails.
#[derive(Debug, Error, Errno)]
pub enum PrivilegeError {
    #[error("the current credential does not have the specified privilege")]
    #[errno(EPERM)]
    NoPrivilege,
}

#[derive(Debug, Error, Errno)]
pub enum PrisonCheckError {
    #[error("insufficient credentials")]
    #[errno(ESRCH)]
    InsuffiecientCredentials,
}
