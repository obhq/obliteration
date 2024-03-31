use super::SysErr;
use crate::fs::{VPath, VPathBuf};
use crate::shm::ShmPath;
use crate::Errno;
use std::ffi::{c_char, CStr};
use std::fmt::{Formatter, LowerHex};
use std::num::{NonZeroI32, TryFromIntError};

/// Input of the syscall entry point.
#[repr(C)]
pub struct SysIn<'a> {
    pub id: u32,
    pub offset: usize,
    pub module: &'a VPathBuf,
    pub args: [SysArg; 6],
}

/// An argument of the syscall.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SysArg(usize);

impl SysArg {
    pub unsafe fn to_path<'a>(self) -> Result<Option<&'a VPath>, SysErr> {
        if self.0 == 0 {
            return Ok(None);
        }

        // TODO: Check maximum path length on the PS4.
        let path = CStr::from_ptr(self.0 as _);
        let path = match path.to_str() {
            Ok(v) => match VPath::new(v) {
                Some(v) => v,
                None => todo!("syscall with non-absolute path {v}"),
            },
            Err(_) => return Err(SysErr::Raw(Errno::ENOENT)),
        };

        Ok(Some(path))
    }

    pub unsafe fn to_shm_path(self) -> Result<Option<ShmPath>, SysErr> {
        match self.0 {
            1 => Ok(Some(ShmPath::Anon)),
            ptr => {
                let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 0x400) };

                let cstr = CStr::from_bytes_until_nul(slice)
                    .map_err(|_| SysErr::Raw(Errno::ENAMETOOLONG))?;

                let path = VPath::new(cstr.to_string_lossy().as_ref())
                    .map(|p| ShmPath::Path(p.to_owned()));

                Ok(path)
            }
        }
    }

    /// See `copyinstr` on the PS4 for a reference.
    pub unsafe fn to_str<'a>(self, max: usize) -> Result<Option<&'a str>, SysErr> {
        if self.0 == 0 {
            return Ok(None);
        }

        let ptr = self.0 as *const c_char;
        let mut len = None;

        for i in 0..max {
            if *ptr.add(i) == 0 {
                len = Some(i);
                break;
            }
        }

        match len {
            Some(i) => Ok(Some(
                std::str::from_utf8(std::slice::from_raw_parts(ptr as _, i)).unwrap(),
            )),
            None => Err(SysErr::Raw(Errno::ENAMETOOLONG)),
        }
    }

    pub fn get(self) -> usize {
        self.0
    }
}

impl<T> From<SysArg> for *const T {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl<T> From<SysArg> for *mut T {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl From<SysArg> for i64 {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl From<SysArg> for u64 {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl From<SysArg> for usize {
    fn from(v: SysArg) -> Self {
        v.0
    }
}

impl TryFrom<SysArg> for i32 {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        TryInto::<u32>::try_into(v.0).map(|v| v as i32)
    }
}

impl TryFrom<SysArg> for Option<NonZeroI32> {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        let v = TryInto::<i32>::try_into(v)?;
        Ok(NonZeroI32::new(v))
    }
}

impl TryFrom<SysArg> for u32 {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        v.0.try_into()
    }
}

impl TryFrom<SysArg> for u8 {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        v.0.try_into()
    }
}

impl PartialEq<usize> for SysArg {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl LowerHex for SysArg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}
