use crate::errno::{Errno, EACCES, EPERM};
use crate::ucred::{Privilege, Ucred};
use std::num::NonZeroI32;
use thiserror::Error;

/// You can map [`None`] to `EPERM` to match with the PS4 behavior.
///
/// See `vfs_unixify_accmode` on the PS4 for a reference.
pub fn unixify_access(mut access: u32) -> Option<u32> {
    // TODO: Refactor this for readability.
    if (access & 0100000) != 0 {
        return Some(0);
    } else if (access & 011000000) != 0 {
        return None;
    } else if (access & 0144010000) != 0 {
        access = (access & 0xfe6fefff) | 010000;
    }

    Some(access & 0xfdb7ffff)
}

/// Returns [`Ok`] if access was granted. The boolean value indicated whether privilege was used to
/// satisfy the request.
///
/// See `vaccess` on the PS4 for a reference.
pub fn check_access(
    cred: &Ucred,
    file_uid: i32,
    file_gid: i32,
    file_mode: u32,
    access: u32,
    is_dir: bool,
) -> Result<bool, AccessError> {
    // TODO: Refactor this for readability.
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
    let exec = ((dac_granted & 0100) == 0) as u8 & ((access as u8) >> 6);
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
        priv_granted |= 0100;
    }

    // Check read privilege.
    if (access & 0400) != 0
        && (dac_granted & 0400) == 0
        && cred.priv_check(Privilege::VFS_READ).is_ok()
    {
        priv_granted |= 0400;
    }

    // Check write privilege.
    if (access & 0200) != 0
        && (dac_granted & 0200) == 0
        && cred.priv_check(Privilege::VFS_WRITE).is_ok()
    {
        priv_granted |= 040000 | 0200;
    }

    // Check admin privilege.
    if (access & 010000) != 0
        && (dac_granted & 010000) == 0
        && cred.priv_check(Privilege::VFS_ADMIN).is_ok()
    {
        priv_granted |= 010000;
    }

    if (!(priv_granted | dac_granted) & access) == 0 {
        Ok(true)
    } else if (access & 010000) != 0 {
        Err(AccessError::NotPermitted)
    } else {
        Err(AccessError::PermissionDenied)
    }
}

/// Represents an error when [`check_access()`] is failed.
#[derive(Debug, Error)]
pub enum AccessError {
    #[error("operation not permitted")]
    NotPermitted,

    #[error("permission denied")]
    PermissionDenied,
}

impl Errno for AccessError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotPermitted => EPERM,
            Self::PermissionDenied => EACCES,
        }
    }
}
