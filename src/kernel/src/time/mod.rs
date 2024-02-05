use crate::errno::EIO;
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

use crate::{
    errno::Errno,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};

pub struct TimeManager {}

impl TimeManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let time = Arc::new(Self {});

        sys.register(116, &time, Self::sys_gettimeofday);

        time
    }

    pub fn sys_gettimeofday(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let tv_out: *mut TimeVal = i.args[0].into();
        let tz_out: *mut TimeZone = i.args[1].into();

        if !tv_out.is_null() {
            unsafe {
                *tv_out = TimeVal::microtime()?;
            }
        }

        if !tz_out.is_null() {
            todo!()
        }

        Ok(SysOut::ZERO)
    }
}

#[repr(C)]
struct TimeVal {
    sec: i64,  // tv_sec
    usec: i64, // tv_usec
}

impl TimeVal {
    #[cfg(unix)]
    fn microtime() -> Result<Self, GetTimeOfDayError> {
        use std::{mem::MaybeUninit, ptr::null_mut};

        let mut tv = MaybeUninit::uninit();

        let res = unsafe { libc::gettimeofday(tv.as_mut_ptr(), null_mut()) };
        if res < 0 {
            Err(std::io::Error::last_os_error())?
        }

        Ok(unsafe { tv.assume_init() }.into())
    }
}

#[cfg(unix)]
impl From<libc::timeval> for TimeVal {
    fn from(tv: libc::timeval) -> Self {
        Self {
            sec: tv.tv_sec,
            usec: tv.tv_usec,
        }
    }
}

#[repr(C)]
struct TimeZone {
    minuteswest: i32, // tz_minuteswest
    dsttime: i32,     // tz_dsttime
}

#[derive(Debug, Error, Errno)]
pub enum GetTimeOfDayError {
    #[error("failed to get time info from libc")]
    #[errno(EIO)]
    LibcFailed(#[from] std::io::Error),
}
