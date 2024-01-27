use crate::errno::EAFNOSUPPORT;
use crate::errno::ESRCH;
use crate::errno::{Errno, EPERM};
use crate::net::AddressFamily;
use crate::ucred::prison::PrisonAllow;
use crate::ucred::prison::PrisonFlags;
use std::num::NonZeroI32;
use std::sync::Arc;
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
    effective_uid: Uid,  // cr_uid
    real_uid: Uid,       // cr_ruid
    groups: Vec<Gid>,    // cr_groups + cr_ngroups
    prison: Arc<Prison>, // cr_prison
    auth: AuthInfo,
}

impl Ucred {
    pub fn new(
        effective_uid: Uid,
        real_uid: Uid,
        mut groups: Vec<Gid>,
        prison: &Arc<Prison>,
        auth: AuthInfo,
    ) -> Self {
        assert!(!groups.is_empty()); // Must have primary group.

        groups[1..].sort_unstable(); // The first one must be primary group.

        Self {
            effective_uid,
            real_uid,
            groups,
            prison: prison.clone(),
            auth,
        }
    }

    pub fn effective_uid(&self) -> Uid {
        self.effective_uid
    }

    pub fn real_uid(&self) -> Uid {
        self.real_uid
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
    pub fn prison_check(&self, other: &Self) -> Result<(), PrisonCheckError> {
        if Arc::ptr_eq(&self.prison, &other.prison) || self.prison.is_child(&other.prison) {
            return Ok(());
        }

        Err(PrisonCheckError::CheckFailed)
    }

    pub fn is_jailed(&self) -> bool {
        Arc::ptr_eq(&self.prison, &PRISON0)
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

    /// See `prison_check_af` on the PS4 for a reference.
    pub fn prison_check_address_family(
        &self,
        family: AddressFamily,
    ) -> Result<(), PrisonCheckAfError> {
        let pr = &self.prison;

        match family {
            AddressFamily::UNIX | AddressFamily::ROUTE => {}
            AddressFamily::INET => todo!(),
            AddressFamily::INET6 => {
                if pr.flags().intersects(PrisonFlags::IP6) {
                    todo!()
                }
            }
            _ => {
                if !pr.allow().intersects(PrisonAllow::ALLOW_SOCKET_AF) {
                    return Err(PrisonCheckAfError::SocketAddressFamilyNotAllowed(family));
                }
            }
        }

        Ok(())
    }
}

/// Represents an error when [`Ucred::priv_check()`] fails.
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

#[derive(Debug, Error)]
pub enum PrisonCheckError {
    #[error("Prison check failed")]
    CheckFailed,
}

impl Errno for PrisonCheckError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::CheckFailed => ESRCH,
        }
    }
}

#[derive(Debug, Error)]
pub enum PrisonCheckAfError {
    #[error("the address family {0} is not allowed by prison")]
    SocketAddressFamilyNotAllowed(AddressFamily),
}

impl Errno for PrisonCheckAfError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::SocketAddressFamilyNotAllowed(_) => EAFNOSUPPORT,
        }
    }
}
