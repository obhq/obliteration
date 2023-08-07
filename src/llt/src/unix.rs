use crate::SpawnError;
use libc::{
    c_int, c_void, pthread_attr_destroy, pthread_attr_init, pthread_attr_t, pthread_create,
    pthread_t,
};
use std::io::Error;
use std::mem::MaybeUninit;

pub unsafe fn spawn(
    stack: *mut u8,
    stack_size: usize,
    entry: extern "C" fn(*mut c_void) -> *mut c_void,
    arg: *mut c_void,
) -> Result<pthread_t, SpawnError> {
    // Initialize thread attribute.
    let mut attr = match PthreadAttr::new() {
        Ok(v) => v,
        Err(e) => return Err(SpawnError::InitAttrFailed(e)),
    };

    // Set stack.
    let err = pthread_attr_setstack(attr.as_mut_ptr(), stack as _, stack_size);

    if err != 0 {
        return Err(SpawnError::SetStackFailed(Error::from_raw_os_error(err)));
    }

    // Create a thread.
    let mut thr = MaybeUninit::<pthread_t>::uninit();
    let err = pthread_create(thr.as_mut_ptr(), attr.as_ptr(), entry, arg);

    if err != 0 {
        Err(SpawnError::CreateThreadFailed(Error::from_raw_os_error(
            err,
        )))
    } else {
        Ok(thr.assume_init())
    }
}

struct PthreadAttr(pthread_attr_t);

impl PthreadAttr {
    fn new() -> Result<Self, Error> {
        let mut attr = MaybeUninit::<pthread_attr_t>::uninit();
        let err = unsafe { pthread_attr_init(attr.as_mut_ptr()) };

        if err != 0 {
            Err(Error::from_raw_os_error(err))
        } else {
            Ok(Self(unsafe { attr.assume_init() }))
        }
    }

    fn as_ptr(&self) -> *const pthread_attr_t {
        &self.0
    }

    fn as_mut_ptr(&mut self) -> *mut pthread_attr_t {
        &mut self.0
    }
}

impl Drop for PthreadAttr {
    fn drop(&mut self) {
        assert_eq!(unsafe { pthread_attr_destroy(&mut self.0) }, 0);
    }
}

extern "C" {
    // This does not available on libc somehow.
    fn pthread_attr_setstack(
        attr: *mut pthread_attr_t,
        stackaddr: *mut c_void,
        stacksize: usize,
    ) -> c_int;
}
