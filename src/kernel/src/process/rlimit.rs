use std::{io::Error, ops::Index};
use thiserror::Error;

//TODO: add remaining limits
#[derive(Debug, Clone, Copy)]
pub enum ResourceType {
    Cpu = 0,
    Fsize = 1,
    Data = 2,
}

#[cfg(target_os = "linux")]
type HostResourceType = libc::__rlimit_resource_t;

#[cfg(target_os = "macos")]
type HostResourceType = libc::c_int;

impl ResourceType {
    #[cfg(unix)]
    pub fn into_host(self) -> HostResourceType {
        match self {
            Self::Cpu => libc::RLIMIT_CPU,
            Self::Fsize => libc::RLIMIT_FSIZE,
            Self::Data => libc::RLIMIT_DATA,
        }
    }
}

#[derive(Debug)]
pub(super) struct Limits([ResourceLimit; Self::NLIMITS]);

impl Limits {
    pub const NLIMITS: usize = 3;

    pub fn load() -> Result<Self, LoadLimitError> {
        use LoadLimitError::*;
        use ResourceType::*;

        let inner = [
            ResourceLimit::try_load(Cpu).map_err(FailedToLoadCpuLimit)?,
            ResourceLimit::try_load(Fsize).map_err(FailedToLoadFsizeLimit)?,
            ResourceLimit::try_load(Data).map_err(FailedToLoadDataLimit)?,
        ];

        Ok(Self(inner))
    }
}

impl Index<ResourceType> for Limits {
    type Output = ResourceLimit;

    fn index(&self, ty: ResourceType) -> &Self::Output {
        self.0.get(ty as usize).unwrap()
    }
}

#[derive(Debug, Error)]
pub enum LoadLimitError {
    #[error("failed to load cpu limit")]
    FailedToLoadCpuLimit(#[source] Error),

    #[error("failed to load fsize limit")]
    FailedToLoadFsizeLimit(#[source] Error),

    #[error("failed to load data limit")]
    FailedToLoadDataLimit(#[source] Error),
}

/// An implementation of `rlimit`.
#[derive(Debug)]
pub struct ResourceLimit {
    cur: usize,
    max: usize,
}

impl ResourceLimit {
    pub(super) fn try_load(ty: ResourceType) -> Result<Self, Error> {
        // TODO: Make sure the value is not exceed the value on the PS4.
        let mut l = Self::host(ty)?;

        if let ResourceType::Data = ty {
            let mb = 1024 * 1024;
            let gb = 1024 * mb;
            let max = 5 * gb;

            if l.max > max {
                l.max = max;
                l.cur = max;
            }
        }

        Ok(l)
    }

    pub fn max(&self) -> usize {
        self.max
    }

    #[cfg(unix)]
    fn host(ty: ResourceType) -> Result<Self, Error> {
        use std::mem::MaybeUninit;

        let mut l = MaybeUninit::uninit();

        if unsafe { libc::getrlimit(ty.into_host(), l.as_mut_ptr()) } < 0 {
            return Err(Error::last_os_error());
        }

        let l = unsafe { l.assume_init() };

        Ok(Self {
            cur: l.rlim_cur.try_into().unwrap(),
            max: l.rlim_max.try_into().unwrap(),
        })
    }

    #[cfg(windows)]
    fn host(ty: ResourceType) -> Result<Self, Error> {
        let (cur, max) = match ty {
            ResourceType::Cpu => (usize::MAX, usize::MAX), // TODO: Get the values from Windows.
            ResourceType::Fsize => (usize::MAX, usize::MAX), // TODO: Get the values from Windows.
            ResourceType::Data => (usize::MAX, usize::MAX), // TODO: Get the values from Windows.
        };

        Ok(Self { cur, max })
    }
}
