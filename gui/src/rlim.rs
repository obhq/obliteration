use std::mem::MaybeUninit;
use thiserror::Error;

pub(crate) fn set_rlimit_nofile() -> Result<(), RlimitError> {
    let mut rlim = MaybeUninit::uninit();

    let ret = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, rlim.as_mut_ptr()) };

    match ret {
        0 => {
            let mut rlim = unsafe { rlim.assume_init() };

            if rlim.rlim_cur < rlim.rlim_max {
                rlim.rlim_cur = rlim.rlim_max;

                if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) } != 0 {
                    return Err(RlimitError::SetRlimitFailed(std::io::Error::last_os_error()));
                }
            }

            Ok(())
        }
        _ => Err(RlimitError::GetRlimitFailed(std::io::Error::last_os_error())),
    }
}

#[derive(Debug, Error)]
pub(crate) enum RlimitError {
    #[error("failed to get RLIMIT_NOFILE -> {0}")]
    GetRlimitFailed(#[source] std::io::Error),

    #[error("failed to set RLIMIT_NOFILE -> {0}")]
    SetRlimitFailed(#[source] std::io::Error),
}
