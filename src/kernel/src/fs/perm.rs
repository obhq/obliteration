use crate::errno::{Errno, EACCES, EPERM};
use crate::ucred::{Gid, Privilege, Ucred, Uid};
use bitflags::bitflags;
use macros::Errno;
use thiserror::Error;

/// You can map [`None`] to `EPERM` to match with the PS4 behavior.
///
/// See `vfs_unixify_accmode` on the PS4 for a reference.
pub fn unixify_access(mut access: Access) -> Option<Access> {
    if access.intersects(Access::EXPLICIT_DENY) {
        return Some(Access::empty());
    } else if access.intersects(Access::DELETE_CHILD | Access::DELETE) {
        return None;
    } else if access.intersects(Access::ADMIN_PERMS) {
        access &= !Access::ADMIN_PERMS;
        access |= Access::ADMIN;
    }

    access &= !(Access::STAT_PERMS | Access::SYNCHRONIZE);

    Some(access)
}

/// Returns [`Ok`] if access was granted. The boolean value indicated whether privilege was used to
/// satisfy the request.
///
/// See `vaccess` on the PS4 for a reference.
pub fn check_access(
    cred: &Ucred,
    file_uid: Uid,
    file_gid: Gid,
    file_mode: Mode,
    access: Access,
    is_dir: bool,
) -> Result<bool, AccessError> {
    // TODO: Refactor this for readability.
    let file_mode: u32 = file_mode.into();
    let access = access.bits();
    let dac_granted = if cred.effective_uid() == file_uid {
        ((file_mode & 0x140) | 0x1000) + if file_mode as i8 > -1 { 0 } else { 0x4080 }
    } else {
        let (v1, v2) = if cred.is_member(file_gid) {
            ((file_mode & 0x28) << 3, file_mode & 0x10)
        } else {
            ((file_mode & 5) << 6, file_mode & 2)
        };

        if v2 == 0 {
            v1
        } else {
            v1 + 0x4080
        }
    };

    if (!dac_granted & access) == 0 {
        return Ok(false);
    }

    // Check exec previlege.
    let mut priv_granted = 0;
    let exec = ((dac_granted & 0o100) == 0) as u8 & ((access as u8) >> 6);
    let pid = if is_dir {
        if exec == 0 {
            None
        } else {
            Some(Privilege::VFS_LOOKUP)
        }
    } else if (file_mode & 0x49) == 0 || exec != 1 {
        None
    } else {
        Some(Privilege::VFS_EXEC)
    };

    if pid.is_some_and(|p| cred.priv_check(p).is_ok()) {
        priv_granted |= 0o100;
    }

    // Check read privilege.
    if (access & 0o400) != 0
        && (dac_granted & 0o400) == 0
        && cred.priv_check(Privilege::VFS_READ).is_ok()
    {
        priv_granted |= 0o400;
    }

    // Check write privilege.
    if (access & 0o200) != 0
        && (dac_granted & 0o200) == 0
        && cred.priv_check(Privilege::VFS_WRITE).is_ok()
    {
        priv_granted |= 0o40000 | 0o200;
    }

    // Check admin privilege.
    if (access & 0o10000) != 0
        && (dac_granted & 0o10000) == 0
        && cred.priv_check(Privilege::VFS_ADMIN).is_ok()
    {
        priv_granted |= 0o10000;
    }

    if (!(priv_granted | dac_granted) & access) == 0 {
        Ok(true)
    } else if (access & 0o10000) != 0 {
        Err(AccessError::NotPermitted)
    } else {
        Err(AccessError::PermissionDenied)
    }
}

/// An implementation of `mode_t`. **Do not accept or pass this struct from/to the PS4 directly**.
///
/// On the PS4 this is `u32`. But some functions in the PS4 use `u16` to represent file mode. The
/// maximum value for file mode, which is `0777` take only 9 bits. So let's use `u16` and don't make
/// this struct representation as a transparent. That mean we can't use this type directly on the
/// function parameter or its return type if that function will be called by the PS4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode(u16);

impl Mode {
    pub const fn new(v: u16) -> Option<Self> {
        if v > 0o777 {
            None
        } else {
            Some(Self(v))
        }
    }
}

impl From<Mode> for u32 {
    fn from(value: Mode) -> Self {
        value.0.into()
    }
}

bitflags! {
    /// An implementation of `accmode_t`.
    /// Some of the constants are unused, but we include them for completeness.
    pub struct Access: u32 {
        const EXEC             = 0o00000000100; // VEXEC
        const WRITE            = 0o00000000200; // VWRITE
        const READ             = 0o00000000400; // VREAD
        const ADMIN            = 0o00000010000; // VADMIN
        const EXPLICIT_DENY    = 0o00000100000; // VEXPLICIT_DENY
        const DELETE_CHILD     = 0o00001000000; // VDELETE_CHILD
        const READ_ATTRIBUTES  = 0o00002000000; // VREAD_ATTRIBUTES
        const WRITE_ATTRIBUTES = 0o00004000000; // VWRITE_ATTRIBUTES
        const DELETE           = 0o00010000000; // VDELETE_CHILD
        const READ_ACL         = 0o00020000000; // VREAD_ACL
        const WRITE_ACL        = 0o00040000000; // VWRITE_ACL
        const WRITE_OWNER      = 0o00100000000; // VWRITE_OWNER
        const SYNCHRONIZE      = 0o00200000000; // VSYNCHRONIZE

        const ADMIN_PERMS = Self::ADMIN.bits() | Self::WRITE_ATTRIBUTES.bits() | Self::WRITE_ACL.bits() | Self::WRITE_OWNER.bits();
        const STAT_PERMS = Self::READ_ATTRIBUTES.bits() | Self::READ_ACL.bits();
    }
}

/// Represents an error when [`check_access()`] is failed.
#[derive(Debug, Error, Errno)]
pub enum AccessError {
    #[error("operation not permitted")]
    #[errno(EPERM)]
    NotPermitted,

    #[error("permission denied")]
    #[errno(EACCES)]
    PermissionDenied,
}
