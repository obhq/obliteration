use std::mem::MaybeUninit;

pub(crate) fn set_rlimit_nofile() {
    {
        let mut rlim = MaybeUninit::uninit();

        let ret = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, rlim.as_mut_ptr()) };

        match ret {
            0 => {
                let mut rlim = unsafe { rlim.assume_init() };

                if rlim.rlim_cur < rlim.rlim_max {
                    rlim.rlim_cur = rlim.rlim_max;

                    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) } != 0 {
                        eprintln!(
                            "Failed to set RLIMIT_NOFILE: {}",
                            std::io::Error::last_os_error()
                        );
                    }
                }
            }
            _ => {
                eprintln!(
                    "Failed to get RLIMIT_NOFILE: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}
