use crate::errno::Errno;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::num::NonZeroI32;
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
    sec: i64,  // tv_sec
    usec: i64, // tv_usec
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
        use windows_sys::Win32::System::{
            SystemInformation::GetSystemTime, Time::SystemTimeToFileTime,
        };

        // The number of hundreds of nanoseconds between the Windows epoch (1601-01-01T00:00:00Z)
        // and the Unix epoch (1970-01-01T00:00:00Z)
        const EPOCH: u64 = 116444736000000000;

        let mut system_time = MaybeUninit::uninit();
        let mut filetime = MaybeUninit::uninit();

        let (system_time, filetime) = unsafe {
            GetSystemTime(system_time.as_mut_ptr());
            let res = SystemTimeToFileTime(system_time.as_ptr(), filetime.as_mut_ptr());

            if res == 0 {
                Err(std::io::Error::last_os_error())?
            }

            (system_time.assume_init(), filetime.assume_init())
        };

        let mut time = 0;

        time += filetime.dwLowDateTime as u64;
        time += (filetime.dwHighDateTime as u64) << 32;

        Ok(Self {
            sec: ((time - EPOCH) / 10_000_000) as i64,
            usec: (system_time.wMilliseconds * 1000) as i64,
        })
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

#[derive(Debug, Error)]
pub enum MicroTimeError {
    #[error("Failed to get time")]
    IoError(#[from] std::io::Error),
}

impl Errno for MicroTimeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::IoError(_) => todo!(),
        }
    }
}
