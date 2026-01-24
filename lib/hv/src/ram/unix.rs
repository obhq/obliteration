use libc::{
    _SC_PAGE_SIZE, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap, munmap, sysconf,
};
use std::io::Error;
use std::num::NonZero;
use std::ptr::null_mut;

pub fn get_page_size() -> Result<NonZero<usize>, Error> {
    let v = unsafe { sysconf(_SC_PAGE_SIZE) };

    if v < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(v.try_into().ok().and_then(NonZero::new).unwrap())
    }
}

pub fn alloc(len: NonZero<usize>) -> Result<*mut u8, Error> {
    let prot = PROT_READ | PROT_WRITE;
    let flags = MAP_PRIVATE | MAP_ANON;
    let mem = unsafe { mmap(null_mut(), len.get(), prot, flags, -1, 0) };

    if mem == MAP_FAILED {
        return Err(Error::last_os_error());
    }

    Ok(mem.cast())
}

pub unsafe fn free(addr: *const u8, len: NonZero<usize>) -> Result<(), Error> {
    if unsafe { munmap(addr.cast_mut().cast(), len.get()) } < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
