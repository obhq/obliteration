use crate::errno::{Errno, EINVAL};
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
        sys.register(234, &time, Self::sys_clock_getres);

        time
    }

    fn sys_gettimeofday(self: &Arc<Self>, _: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let tv: *mut TimeVal = i.args[0].into();
        let tz: *mut TimeZone = i.args[1].into();

        info!("Getting the time of day");

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

    fn sys_clock_gettime(self: &Arc<Self>, _: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let clock_id: ClockId = {
            let clock: i32 = i.args[0].try_into().unwrap();

            clock.try_into()?
        };

        let ts: *mut TimeSpec = i.args[1].into();

        info!("Getting clock time with clock_id = {clock_id:?}");

        unsafe {
            *ts = match clock_id {
                ClockId::REALTIME | ClockId::REALTIME_PRECISE => todo!(),
                ClockId::VIRTUAL => todo!(),
                ClockId::PROF => todo!(),
                ClockId::UPTIME
                | ClockId::UPTIME_PRECISE
                | ClockId::MONOTONIC
                | ClockId::MONOTONIC_PRECISE => Self::nanouptime()?,
                ClockId::UPTIME_FAST | ClockId::MONOTONIC_FAST => todo!(),
                ClockId::REALTIME_FAST => todo!(),
                ClockId::SECOND => Self::time_second()?,
                ClockId::THREAD_CPUTIME_ID => todo!(),
                ClockId::PROC_TIME => todo!(),
                ClockId::EXT_NETWORK => todo!(),
                ClockId::EXT_DEBUG_NETWORK => todo!(),
                ClockId::EXT_AD_NETWORK => todo!(),
                ClockId::EXT_RAW_NETWORK => todo!(),
                _ => todo!("clock_gettime with sclock_id = {clock_id:?}"),
            }
        }

        Ok(SysOut::ZERO)
    }

    fn sys_clock_getres(self: &Arc<Self>, _: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let clock_id: ClockId = {
            let clock: i32 = i.args[0].try_into().unwrap();

            clock.try_into()?
        };
        let ts: *mut TimeSpec = i.args[1].into();

        info!("Getting clock resolution with clock_id = {clock_id:?}");

        if !ts.is_null() {
            unsafe {
                *ts = match clock_id {
                    ClockId::REALTIME
                    | ClockId::MONOTONIC
                    | ClockId::UPTIME
                    | ClockId::UPTIME_PRECISE
                    | ClockId::UPTIME_FAST
                    | ClockId::REALTIME_PRECISE
                    | ClockId::REALTIME_FAST
                    | ClockId::MONOTONIC_PRECISE
                    | ClockId::MONOTONIC_FAST => todo!(),
                    ClockId::VIRTUAL | ClockId::PROF => todo!(),
                    ClockId::SECOND => TimeSpec { sec: 1, nsec: 0 },
                    ClockId::THREAD_CPUTIME_ID => todo!(),
                    _ => return Ok(SysOut::ZERO),
                }
            }
        }

        Ok(SysOut::ZERO)
    }

    #[cfg(unix)]
    fn nanouptime() -> Result<TimeSpec, NanoUpTimeError> {
        use libc::{clock_gettime, CLOCK_MONOTONIC};
        use std::mem::MaybeUninit;

        let mut ts = MaybeUninit::uninit();

        let res = unsafe { clock_gettime(CLOCK_MONOTONIC, ts.as_mut_ptr()) };

        if res < 0 {
            Err(std::io::Error::last_os_error())?;
        }

        Ok(unsafe { ts.assume_init() }.into())
    }

    #[cfg(windows)]
    pub fn nanouptime() -> Result<TimeSpec, NanoUpTimeError> {
        use windows_sys::Win32::System::Performance::{
            QueryPerformanceCounter, QueryPerformanceFrequency,
        };

        let mut counter = 0;
        let mut frequency = 0;

        unsafe {
            if QueryPerformanceCounter(&mut counter) == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if QueryPerformanceFrequency(&mut frequency) == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }

        // Convert the counter to nanoseconds (Ensure no overflow using leftover ticks.)
        let seconds = counter / frequency;
        let leftover_ticks = counter % frequency;
        let nanoseconds = (leftover_ticks * 1_000_000_000) / frequency;

        Ok(TimeSpec {
            sec: seconds as i64,
            nsec: nanoseconds as i64,
        })
    }

    pub fn time_second() -> Result<TimeSpec, NanoUpTimeError> {
        Ok(TimeSpec {
            nsec: 0,
            ..Self::nanouptime()?
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClockId(i32);

impl ClockId {
    pub const REALTIME: Self = Self(0);
    pub const VIRTUAL: Self = Self(1);
    pub const PROF: Self = Self(2);
    pub const MONOTONIC: Self = Self(4);
    pub const UPTIME: Self = Self(5);
    pub const UPTIME_PRECISE: Self = Self(7);
    pub const UPTIME_FAST: Self = Self(8);
    pub const REALTIME_PRECISE: Self = Self(9);
    pub const REALTIME_FAST: Self = Self(10);
    pub const MONOTONIC_PRECISE: Self = Self(11);
    pub const MONOTONIC_FAST: Self = Self(12);
    pub const SECOND: Self = Self(13);
    pub const THREAD_CPUTIME_ID: Self = Self(14);
    pub const PROC_TIME: Self = Self(15);
    pub const EXT_NETWORK: Self = Self(16);
    pub const EXT_DEBUG_NETWORK: Self = Self(17);
    pub const EXT_AD_NETWORK: Self = Self(18);
    pub const EXT_RAW_NETWORK: Self = Self(19);
}

impl TryFrom<i32> for ClockId {
    type Error = SysErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let clock_id = match value {
            0 => Self::REALTIME,
            1 => Self::VIRTUAL,
            2 => Self::PROF,
            4 => Self::MONOTONIC,
            5 => Self::UPTIME,
            7 => Self::UPTIME_PRECISE,
            8 => Self::UPTIME_FAST,
            9 => Self::REALTIME_PRECISE,
            10 => Self::REALTIME_FAST,
            11 => Self::MONOTONIC_PRECISE,
            12 => Self::MONOTONIC_FAST,
            13 => Self::SECOND,
            14 => Self::THREAD_CPUTIME_ID,
            15 => Self::PROC_TIME,
            16 => Self::EXT_NETWORK,
            17 => Self::EXT_DEBUG_NETWORK,
            18 => Self::EXT_AD_NETWORK,
            19 => Self::EXT_RAW_NETWORK,
            ..=-1 => Self(value),
            _ => return Err(SysErr::Raw(EINVAL)),
        };

        Ok(clock_id)
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
