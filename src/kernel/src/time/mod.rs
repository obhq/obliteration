use crate::errno::AsErrno;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

pub struct TimeManager {}

impl TimeManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let time = Arc::new(Self {});

        sys.register(116, &time, Self::sys_gettimeofday);
        sys.register(232, &time, Self::sys_clock_gettime);

        time
    }

    pub fn sys_gettimeofday(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let tv: *mut TimeVal = i.args[0].into();
        let tz: *mut TimeZone = i.args[1].into();

        if !tv.is_null() {
            unsafe {
                *tv = TimeVal::microtime()?;
            }
        }

        if !tz.is_null() {
            todo!()
        }

        Ok(SysOut::ZERO)
    }

    pub fn sys_clock_gettime(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let clock: i32 = i.args[0].try_into().unwrap();

        todo!("clock_gettime with clock = {clock}")
    }
}

/// An implementation of the `timespec` structure.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimeSpec {
    sec: i64,
    nsec: i64,
}

impl TimeSpec {
    pub fn now() -> Self {
        TimeVal::microtime().expect("Couldn't get time").into()
    }
}

impl From<TimeVal> for TimeSpec {
    fn from(tv: TimeVal) -> Self {
        Self {
            sec: tv.sec,
            nsec: tv.usec * 1000,
        }
    }
}

#[repr(C)]
struct TimeVal {
    sec: i64,  // tv_sec (seconds)
    usec: i64, // tv_usec (microseconds)
}

impl TimeVal {
    #[cfg(unix)]
    fn microtime() -> Result<Self, MicroTimeError> {
        use std::{mem::MaybeUninit, ptr::null_mut};

        let mut tv = MaybeUninit::uninit();

        let res = unsafe { libc::gettimeofday(tv.as_mut_ptr(), null_mut()) };
        if res < 0 {
            Err(std::io::Error::last_os_error())?
        }

        Ok(unsafe { tv.assume_init() }.into())
    }

    #[cfg(windows)]
    fn microtime() -> Result<Self, MicroTimeError> {
        use std::mem::MaybeUninit;
        use windows_sys::Win32::System::SystemInformation::GetSystemTimePreciseAsFileTime;

        let mut file_time = MaybeUninit::uninit();

        unsafe {
            GetSystemTimePreciseAsFileTime(file_time.as_mut_ptr());
        }

        let file_time = unsafe { file_time.assume_init() };

        let intervals = (file_time.dwHighDateTime as u64) << 32 | file_time.dwLowDateTime as u64;

        let sec = (intervals / 10_000_000 - 11_644_473_600)
            .try_into()
            .unwrap();
        let usec = (intervals % 10_000_000).try_into().unwrap();

        Ok(Self { sec, usec })
    }
}

#[cfg(unix)]
impl From<libc::timeval> for TimeVal {
    fn from(tv: libc::timeval) -> Self {
        Self {
            sec: tv.tv_sec,
            usec: tv.tv_usec as i64, // The cast is here because of MacOS
        }
    }
}

#[repr(C)]
struct TimeZone {
    minuteswest: i32, // tz_minuteswest
    dsttime: i32,     // tz_dsttime
}

#[derive(Debug, Error, Errno)]
pub enum MicroTimeError {
    #[error("Failed to get time")]
    #[errno(EIO)]
    IoError(#[from] std::io::Error),
}
