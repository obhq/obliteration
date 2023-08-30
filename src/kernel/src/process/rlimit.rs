use std::io::Error;

/// An implementation of `rlimit`.
#[derive(Debug)]
pub struct ResourceLimit {
    cur: usize,
    max: usize,
}

impl ResourceLimit {
    pub const CPU: usize = 0;
    pub const FSIZE: usize = 1;
    pub const DATA: usize = 2;
    pub const NLIMITS: usize = 3;

    pub(super) fn new(ty: usize) -> Result<Self, Error> {
        // TODO: Make sure the value is not exceed the value on the PS4.
        let mut l = Self::host(ty)?;

        match ty {
            Self::DATA => {
                let mb = 1024 * 1024;
                let gb = mb * 1024;
                let max = gb * 5;

                if l.max > max {
                    l.max = max;
                    l.cur = max;
                }
            }
            _ => {}
        }

        Ok(l)
    }

    pub fn max(&self) -> usize {
        self.max
    }

    #[cfg(unix)]
    fn host(ty: usize) -> Result<Self, Error> {
        use std::mem::MaybeUninit;

        let mut l = MaybeUninit::uninit();
        let r = match ty {
            Self::CPU => libc::RLIMIT_CPU,
            Self::FSIZE => libc::RLIMIT_FSIZE,
            Self::DATA => libc::RLIMIT_DATA,
            v => todo!("ResourceLimit::new({v})"),
        };

        if unsafe { libc::getrlimit(r, l.as_mut_ptr()) } < 0 {
            return Err(Error::last_os_error());
        }

        let l = unsafe { l.assume_init() };

        Ok(Self {
            cur: l.rlim_cur.try_into().unwrap(),
            max: l.rlim_max.try_into().unwrap(),
        })
    }

    #[cfg(windows)]
    fn host(ty: usize) -> Result<Self, Error> {
        let (cur, max) = match ty {
            Self::CPU => (u64::MAX, u64::MAX), // TODO: Get the values from Windows.
            Self::FSIZE => (u64::MAX, u64::MAX), // TODO: Get the values from Windows.
            Self::DATA => (u64::MAX, u64::MAX), // TODO: Get the values from Windows.
            v => todo!("ResourceLimit::new({v})"),
        };

        Ok(Self { cur, max })
    }
}
