pub use self::auth::*;
pub use self::id::*;
pub use self::privilege::*;
use crate::errno::{Errno, EPERM};
use crate::rcmgr::RcMgr;
use macros::Errno;
use thiserror::Error;

mod auth;
mod id;
mod privilege;

/// An implementation of `ucred` structure.
#[derive(Debug, Clone)]
pub struct Ucred {
    effective_uid: Uid, // cr_uid
    real_uid: Uid,      // cr_ruid
    groups: Vec<Gid>,   // cr_groups + cr_ngroups
    auth: AuthInfo,
}

impl Ucred {
    pub fn new(effective_uid: Uid, real_uid: Uid, mut groups: Vec<Gid>, auth: AuthInfo) -> Self {
        assert!(!groups.is_empty()); // Must have primary group.

        groups[1..].sort_unstable(); // The first one must be primary group.

        Self {
            effective_uid,
            real_uid,
            groups,
            auth,
        }
    }

    pub fn effective_uid(&self) -> Uid {
        self.effective_uid
    }

    pub fn real_uid(&self) -> Uid {
        self.real_uid
    }

    pub fn groups(&self) -> &[Gid] {
        &self.groups
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

    pub fn is_libkernel_web(&self) -> bool {
        // TODO: Refactor this for readability.
        let val = self.auth.paid.get().wrapping_add(0xc7ffffffeffffffc);
        (val < 0xf) && ((0x6001 >> (val & 0x3f) & 1) != 0)
    }

    pub fn is_webprocess_webapp_or_webmas(&self) -> bool {
        matches!(
            self.auth.paid.get(),
            0x380000001000000f | 0x3800000010000013
        )
    }

    /// See `sceSblACMgrIsDiskplayeruiProcess` on the PS4 for a reference.
    pub fn is_diskplayerui_process(&self) -> bool {
        self.auth.paid.get() == 0x3800000010000009
    }

    /// See `sceSblACMgrIsJitCompilerProcess` on the PS4 for a reference.
    pub fn is_jit_compiler_process(&self) -> bool {
        let val = self.auth.caps.0[1];

        if val >> 0x3e & 1 == 0 {
            if val >> 0x38 & 1 != 0 && !todo!() {
                true
            } else if (self.auth.paid.get() >> 56) == 0x31 && !todo!() {
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    /// See `sceSblACMgrIsJitApplicationProcess` on the PS4 for a reference.
    pub fn is_jit_application_process(&self) -> bool {
        let val = self.auth.caps.0[1];

        if val >> 0x3d & 1 == 0 {
            if val >> 0x38 & 1 != 0 && !todo!() {
                true
            } else if (self.auth.paid.get() >> 56) == 0x31 && !todo!() {
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    /// See `sceSblACMgrIsVideoplayerProcess` on the PS4 for a reference.
    pub fn is_videoplayer_process(&self) -> bool {
        self.auth.paid.get().wrapping_add(0xc7ffffffefffffff) < 2
    }

    /// See `sceSblACMgrHasUseVideoServiceCapability` on the PS4 for a reference.
    pub fn has_use_video_service_capability(&self) -> bool {
        self.auth.caps.has_use_video_service()
    }

    /// See `sceSblACMgrIsWebcoreProcess` on the PS4 for a reference.
    pub fn is_webcore_process(&self) -> bool {
        let val = self.auth.paid.get().wrapping_add(0xc7ffffffeffffffd);

        (val < 0x11) && (0x1d003 >> (val & 0x3f) & 1 != 0)
    }

    /// See `sceSblACMgrHasSceProgramAttribute` on the PS4 for a reference.
    pub fn has_sce_program_attribute(&self) -> bool {
        self.auth.attrs.has_sce_program_attribute()
    }

    /// See `sceSblACMgrIsDebuggableProcess` on the PS4 for a reference.
    pub fn is_debuggable_process(&self, rc: &RcMgr) -> bool {
        self.auth.attrs.is_debuggable_process(rc)
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
#[derive(Debug, Error, Errno)]
pub enum PrivilegeError {
    #[error("the current credential does not have the specified privilege")]
    #[errno(EPERM)]
    NoPrivilege,
}
