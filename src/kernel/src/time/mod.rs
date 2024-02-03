use crate::errno::{Errno, EINVAL};
use crate::info;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::io::Error;
use std::mem::zeroed;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;
pub struct TimeManager {}

impl TimeManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let time = Arc::new(Self {});

        sys.register(116, &time, Self::sys_gettimeofday);
        sys.register(232, &time, Self::sys_clock_gettime);
        sys.register(240, &time, Self::sys_nanosleep);

        time
    }

    pub fn sys_gettimeofday(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
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

    pub fn sys_clock_gettime(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let clock: i32 = i.args[0].try_into().unwrap();

        todo!("clock_gettime with clock = {clock}")
    }

    fn sys_nanosleep(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let req: *const Timespec = i.args[0].try_into().unwrap();
        let rem: *mut Timespec = i.args[1].try_into().unwrap();

        let sleep = unsafe { &*req };

        info!("sys_nanosleep({:#?}, {:?})", sleep, rem);

        if sleep.seconds < 0 || sleep.nanoseconds < 0 || sleep.nanoseconds > 999999999 {
            return Err(SysErr::Raw(EINVAL));
        }

        if !rem.is_null() {
            unsafe { *rem = zeroed() }
        }

        match Timespec::raw_nanosleep(sleep) {
            Err(error) => Err(SysErr::Raw(unsafe {
                NonZeroI32::new_unchecked(error.raw_os_error().unwrap())
            })),
            Ok(_) => Ok(0.into()),
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
#[derive(Debug)]
pub struct Timespec {
    pub seconds: i64,     // tv_sec
    pub nanoseconds: i64, // tv_nsec
}

impl Timespec {
    #[cfg(unix)]
    fn raw_nanosleep(ts: &Timespec) -> Result<std::ffi::c_int, Error> {
        let mut req = ::libc::timespec {
            tv_sec: ts.seconds,
            tv_nsec: ts.nanoseconds,
        };

        let mut rem = unsafe { zeroed() };

        loop {
            let ret = unsafe { ::libc::nanosleep(&req, &mut rem) };

            if ret == 0 {
                return Ok(0.into());
            }

            if Error::last_os_error().raw_os_error().unwrap() == ::libc::EINTR {
                req = rem;
                rem = unsafe { zeroed() }
            } else {
                return Err(Error::last_os_error());
            }
        }
    }

    #[cfg(windows)]
    fn raw_nanosleep(ts: &Timespec) -> Result<std::ffi::c_int, Error> {
        use std::os::raw::c_void;
        use std::sync::{Arc, Condvar, Mutex};
        use windows_sys::Win32::Foundation::{BOOLEAN, HANDLE};
        use windows_sys::Win32::System::Threading::{CreateTimerQueueTimer, WT_EXECUTEONLYONCE};

        pub type Timer = HANDLE;

        let pair: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = pair.clone();

        unsafe extern "system" fn callback(arg: *mut ::core::ffi::c_void, _: BOOLEAN) {
            let pair2: Arc<(Mutex<bool>, Condvar)> = Arc::from_raw(arg as _);
            let (lock, cvar) = &*pair2;
            let mut triggered = lock.lock().unwrap();
            *triggered = true;
            cvar.notify_one();
        }

        let milliseconds = ts.seconds * 1000 + ts.nanoseconds / 1000000;

        let mut timerhandle = HANDLE::default();
        let ret = unsafe {
            CreateTimerQueueTimer(
                &mut timerhandle,
                0,
                Some(callback),
                Arc::into_raw(pair2) as *const c_void,
                milliseconds as u32,
                0,
                WT_EXECUTEONLYONCE,
            )
        };

        let (lock, cvar) = &*pair;
        let mut triggered = lock.lock().unwrap();
        while !*triggered {
            triggered = cvar.wait(triggered).unwrap();
        }

        if ret > 0 {
            return Ok(0.into());
        } else {
            return Err(Error::last_os_error());
        }
    }
}

#[cfg(unix)]
impl From<libc::timespec> for Timespec {
    fn from(ts: libc::timespec) -> Self {
        Self {
            seconds: ts.tv_sec,
            nanoseconds: ts.tv_nsec,
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
