use crate::errno::Errno;
use crate::info;
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
        let clock: Clock = {
            let clock: i32 = i.args[0].try_into().unwrap();

            clock.try_into()?
        };

        let timespec: *mut TimeSpec = i.args[1].into();

        info!("Getting clock time with clock_id = {clock:?}");

        unsafe {
            *timespec = match clock {
                Clock::Monotonic => Self::nanouptime()?,
            }
        }

        Ok(SysOut::ZERO)
    }

    #[cfg(unix)]
    pub fn nanouptime() -> Result<TimeSpec, NanoUpTimeError> {
        use libc::clock_gettime;
        use std::mem::MaybeUninit;

        let mut ts = MaybeUninit::uninit();

        let res = unsafe { clock_gettime(libc::CLOCK_MONOTONIC, ts.as_mut_ptr()) };

        if res < 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        Ok(unsafe { ts.assume_init() }.into())
    }

    #[cfg(windows)]
    pub fn nanouptime() -> Result<TimeSpec, NanoUpTimeError> {
        todo!()
    }
}

#[derive(Debug)]
enum Clock {
    Monotonic = 4,
}

impl TryFrom<i32> for Clock {
    type Error = SysErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            4 => Ok(Self::Monotonic),
            _ => todo!(),
        }
    }
}

/// An implementation of the `timespec` structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TimeSpec {
    sec: i64,
    nsec: i64,
}

impl TimeSpec {
    pub fn now() -> Self {
        TimeVal::microtime().expect("Couldn't get time").into()
    }
}

#[cfg(unix)]
impl From<libc::timespec> for TimeSpec {
    fn from(ts: libc::timespec) -> Self {
        Self {
            sec: ts.tv_sec,
            nsec: ts.tv_nsec,
        }
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

#[derive(Debug, Error)]
pub enum NanoUpTimeError {
    #[error("Failed to get time")]
    IoError(#[from] std::io::Error),
}

impl Errno for NanoUpTimeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::IoError(_) => todo!(),
        }
    }
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
